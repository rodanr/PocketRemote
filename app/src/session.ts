// Phone-side protocol state machines (PLAN.md §3.4, §3.5, §4.1). Plain TypeScript: no React
// Native imports. The socket and timers live in the UI layer; this file only sees frames.
// Security-sensitive: changes here need the human's review. Never log keys, tokens or payloads.
import { DecodeError, decodeMessage, encodeMessage, isHostToPhone, isPhoneToHost } from './decode';
import {
  HANDSHAKE_VERSION,
  HandshakeState,
  KIND_PAIRING,
  KIND_SESSION,
  KK_PATTERN,
  MAX_HANDSHAKE_FRAME,
  NoiseError,
  Transport,
  XXPSK3_PATTERN,
  equalBytes,
  pairingCode,
  type Keypair,
  type Rng,
} from './noise';
import { base64urlDecode } from './base64url';
import type { Message } from './protocol';

/** The phone pings the host this often; the host drops silent sessions after 15 s. */
export const PING_INTERVAL_MS = 5000;


function handshakeFrame(kind: number, noiseMessage: Uint8Array): Uint8Array {
  const out = new Uint8Array(2 + noiseMessage.length);
  out[0] = kind;
  out[1] = HANDSHAKE_VERSION;
  out.set(noiseMessage, 2);
  if (out.length > MAX_HANDSHAKE_FRAME) throw new NoiseError();
  return out;
}

function splitHandshakeFrame(frame: Uint8Array, expectedKind: number): Uint8Array {
  if (frame.length < 3 || frame.length > MAX_HANDSHAKE_FRAME) throw new NoiseError();
  if (frame[0] !== expectedKind || frame[1] !== HANDSHAKE_VERSION) throw new NoiseError();
  return frame.subarray(2);
}

function readEmpty(hs: HandshakeState, body: Uint8Array): void {
  // Handshake payloads are unused: anything non-empty is a protocol violation.
  if (hs.readMessage(body).length !== 0) throw new NoiseError();
}

export type SessionEvent =
  | { kind: 'ready' }
  | { kind: 'message'; message: Message }
  /** The session must end: close the socket. `reason` is for UI only and carries no detail. */
  | { kind: 'closed'; reason: 'protocol' };

export interface SessionOptions {
  rng: Rng;
  /** The phone's static keypair (from secure storage). */
  phone: Keypair;
  /** The pinned host static public key (from pairing). */
  hostPublic: Uint8Array;
  /** Transmit one binary WebSocket frame. */
  send: (frame: Uint8Array) => void;
  onEvent: (event: SessionEvent) => void;
}

/** `Noise_KK` session as initiator. Feed it every binary frame; it emits events. */
export class SessionClient {
  state: 'idle' | 'handshaking' | 'ready' | 'closed' = 'idle';
  private hs: HandshakeState | null = null;
  private transport: Transport | null = null;

  constructor(private readonly o: SessionOptions) {}

  /** Send the first handshake message. Call once, when the socket opens. */
  start(): void {
    if (this.state !== 'idle') return this.fail();
    try {
      this.hs = new HandshakeState({
        pattern: KK_PATTERN,
        role: 'initiator',
        rng: this.o.rng,
        s: this.o.phone,
        rs: this.o.hostPublic,
      });
      this.state = 'handshaking';
      this.o.send(handshakeFrame(KIND_SESSION, this.hs.writeMessage()));
    } catch {
      this.fail();
    }
  }

  receive(frame: Uint8Array): void {
    try {
      if (this.state === 'handshaking' && this.hs) {
        readEmpty(this.hs, splitHandshakeFrame(frame, KIND_SESSION));
        const { send, recv } = this.hs.split();
        this.transport = new Transport(send, recv);
        this.hs = null;
        this.state = 'ready';
        this.o.onEvent({ kind: 'ready' });
      } else if (this.state === 'ready' && this.transport) {
        const message = decodeMessage(this.transport.decrypt(frame));
        if (!isHostToPhone(message)) throw new DecodeError();
        this.o.onEvent({ kind: 'message', message });
      } else {
        throw new NoiseError();
      }
    } catch {
      this.fail();
    }
  }

  /** Send a command. Only phone-to-host message types are allowed. */
  send(message: Message): void {
    if (this.state !== 'ready' || !this.transport || !isPhoneToHost(message)) return this.fail();
    try {
      this.o.send(this.transport.encrypt(encodeMessage(message)));
    } catch {
      this.fail();
    }
  }

  /** The socket closed on its own (not our doing). */
  remoteClosed(): void {
    this.state = 'closed';
    this.hs = null;
    this.transport = null;
  }

  private fail(): void {
    if (this.state === 'closed') return;
    this.remoteClosed();
    this.o.onEvent({ kind: 'closed', reason: 'protocol' });
  }
}

// ---------------------------------------------------------------- pairing

export interface PairingOffer {
  /** `ip:port` or `[ipv6]:port`. */
  addr: string;
  token: Uint8Array;
  hostKey: Uint8Array;
  hostId: string;
}

export const QR_SCHEME = 'pocketremote';

const ADDR = /^(?:\d{1,3}(?:\.\d{1,3}){3}|\[[0-9a-fA-F:.]+\]):(\d{1,5})$/;

/** Strict parser for the QR content `pocketremote://pair?v=1&addr=..&pt=..&hk=..&id=..`. */
export function parsePairingUri(uri: string): PairingOffer | null {
  const prefix = `${QR_SCHEME}://pair?`;
  if (!uri.startsWith(prefix) || uri.length > 512) return null;
  const params = new Map<string, string>();
  for (const pair of uri.slice(prefix.length).split('&')) {
    const eq = pair.indexOf('=');
    if (eq <= 0) return null;
    const k = pair.slice(0, eq);
    if (params.has(k)) return null;
    params.set(k, pair.slice(eq + 1));
  }
  if (params.size !== 5 || params.get('v') !== '1') return null;
  const addr = params.get('addr') ?? '';
  const m = ADDR.exec(addr);
  const port = m ? Number(m[1]) : 0;
  if (!m || port < 1 || port > 65535) return null;
  const token = base64urlDecode(params.get('pt') ?? '');
  const hostKey = base64urlDecode(params.get('hk') ?? '');
  const hostId = params.get('id') ?? '';
  if (!token || token.length !== 32 || !hostKey || hostKey.length !== 32) return null;
  if (!/^[0-9a-f]{16}$/.test(hostId)) return null;
  return { addr, token, hostKey, hostId };
}

export type PairingEvent =
  /** Handshake done and the host key matches the QR. Show the code; the user approves on the Mac. */
  | { kind: 'code'; code: string }
  /** The host approved. Store `hostKey` and the address. */
  | { kind: 'paired'; hostName: string; hostKey: Uint8Array }
  | { kind: 'failed' };

export interface PairingOptions {
  rng: Rng;
  phone: Keypair;
  offer: PairingOffer;
  send: (frame: Uint8Array) => void;
  onEvent: (event: PairingEvent) => void;
}

/** `Noise_XXpsk3` pairing as initiator. */
export class PairingClient {
  state: 'idle' | 'msg1' | 'msg3' | 'awaiting' | 'done' | 'failed' = 'idle';
  private hs: HandshakeState | null = null;
  private transport: Transport | null = null;

  constructor(private readonly o: PairingOptions) {}

  start(): void {
    if (this.state !== 'idle') return this.fail();
    try {
      this.hs = new HandshakeState({
        pattern: XXPSK3_PATTERN,
        role: 'initiator',
        rng: this.o.rng,
        s: this.o.phone,
        psk: this.o.offer.token,
      });
      this.state = 'msg1';
      this.o.send(handshakeFrame(KIND_PAIRING, this.hs.writeMessage()));
    } catch {
      this.fail();
    }
  }

  receive(frame: Uint8Array): void {
    try {
      if (this.state === 'msg1' && this.hs) {
        readEmpty(this.hs, splitHandshakeFrame(frame, KIND_PAIRING));
        // The host's static key from the handshake MUST equal `hk` from the QR.
        const seen = this.hs.remoteStatic();
        if (!seen || !equalBytes(seen, this.o.offer.hostKey)) throw new NoiseError();
        this.o.send(handshakeFrame(KIND_PAIRING, this.hs.writeMessage()));
        const code = pairingCode(this.hs.handshakeHash());
        const { send, recv } = this.hs.split();
        this.transport = new Transport(send, recv);
        this.hs = null;
        this.state = 'awaiting';
        this.o.onEvent({ kind: 'code', code });
      } else if (this.state === 'awaiting' && this.transport) {
        const message = decodeMessage(this.transport.decrypt(frame));
        if (message.type !== 'paired') throw new DecodeError();
        this.state = 'done';
        this.o.onEvent({ kind: 'paired', hostName: message.host_name, hostKey: this.o.offer.hostKey });
      } else {
        throw new NoiseError();
      }
    } catch {
      this.fail();
    }
  }

  /** The socket closed. Before `paired` arrived, that means rejected, expired or failed. */
  remoteClosed(): void {
    if (this.state !== 'done') this.fail();
  }

  private fail(): void {
    if (this.state === 'failed' || this.state === 'done') return;
    this.state = 'failed';
    this.hs = null;
    this.transport = null;
    this.o.onEvent({ kind: 'failed' });
  }
}

