// Connection controller (PLAN.md §6): connects, reconnects with backoff, pings, and notices a
// silent drop. Plain TypeScript with injected sockets and timers. Security-sensitive: it owns the
// session lifecycle. Never log keys or payloads.
import { PING_INTERVAL_MS, SessionClient, type SessionEvent } from './session';
import type { Keypair, Rng } from './noise';
import type { ErrorCode, Message, Platform } from './protocol';

export interface SocketLike {
  binaryType: string;
  onopen: (() => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onclose: ((ev: unknown) => void) | null;
  onerror: ((ev: unknown) => void) | null;
  send(data: Uint8Array): void;
  close(): void;
}

export interface Timers {
  setTimeout(fn: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
  now(): number;
}

export interface ConnectionDeps {
  createSocket(url: string): SocketLike;
  timers: Timers;
  rng: Rng;
}

export interface Credentials {
  /** `ip:port` or `[ipv6]:port`. */
  addr: string;
  phone: Keypair;
  hostKey: Uint8Array;
}

export type Phase = 'connecting' | 'connected' | 'waiting' | 'stopped';
/** Why we are not connected, in words the UI can show. */
/** `rejected`: we reached the computer but could not complete the secure handshake, so the pairing is no longer valid. */
export type Reason = 'unreachable' | 'lost' | 'rejected' | ErrorCode;

export interface HostStatus {
  platform: Platform;
  profile: string;
  profiles: string[];
  paused: boolean;
}

export interface ConnState {
  phase: Phase;
  reason?: Reason;
  /** Failed attempts in a row since the last good connection. */
  attempt: number;
  /** After repeated failures the host may have moved: offer to scan the QR again. */
  needsRescan: boolean;
  status?: HostStatus;
}

export const BACKOFF_START_MS = 250;
export const BACKOFF_MAX_MS = 5000;
export const SILENCE_LIMIT_MS = 15000;
export const RESCAN_AFTER_ATTEMPTS = 6;
/** Reaching the host but failing the handshake this many times in a row means the pairing is stale: retrying cannot help. */
export const REJECTED_AFTER = 2;

export function backoffDelay(attempt: number): number {
  return Math.min(BACKOFF_START_MS * 2 ** attempt, BACKOFF_MAX_MS);
}

export class RemoteConnection {
  private state: ConnState = { phase: 'waiting', attempt: 0, needsRescan: false };
  private listeners = new Set<(s: ConnState) => void>();
  private socket: SocketLike | null = null;
  private session: SessionClient | null = null;
  private retryTimer: unknown = null;
  private pingTimer: unknown = null;
  private lastRx = 0;
  private hostError: ErrorCode | null = null;
  private stopped = true;
  private generation = 0;
  private opened = false;
  private handshakeFailures = 0;

  constructor(
    private readonly deps: ConnectionDeps,
    private readonly creds: Credentials,
  ) {}

  getState(): ConnState {
    return this.state;
  }

  subscribe(fn: (s: ConnState) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  start(): void {
    this.stopped = false;
    this.state = { ...this.state, attempt: 0, needsRescan: false };
    this.handshakeFailures = 0;
    this.connect();
  }

  /** The app came to the foreground or the user tapped reconnect: try again right away. */
  reconnectNow(): void {
    if (this.state.phase === 'connected' || this.state.phase === 'connecting') return;
    this.stopped = false;
    this.clearRetry();
    // Keep needsRescan: pressing Reconnect must not hide "Scan again" while the pairing is still broken.
    // Only a successful connection clears it.
    this.state = { ...this.state, attempt: 0 };
    this.connect();
  }

  stop(): void {
    this.stopped = true;
    this.teardown();
    this.set({ phase: 'stopped', reason: undefined });
  }

  /** Send a command. Returns false (and sends nothing) unless the session is up. */
  send(message: Message): boolean {
    if (this.state.phase !== 'connected' || !this.session) return false;
    this.session.send(message);
    return true;
  }

  private set(patch: Partial<ConnState>): void {
    this.state = { ...this.state, ...patch };
    for (const l of this.listeners) l(this.state);
  }

  private connect(): void {
    this.teardown();
    const gen = ++this.generation;
    this.hostError = null;
    this.opened = false;
    this.set({ phase: 'connecting', reason: undefined });
    let socket: SocketLike;
    try {
      socket = this.deps.createSocket(`ws://${this.creds.addr}/ws`);
    } catch {
      this.handleClose(gen);
      return;
    }
    socket.binaryType = 'arraybuffer';
    this.socket = socket;
    const session = new SessionClient({
      rng: this.deps.rng,
      phone: this.creds.phone,
      hostPublic: this.creds.hostKey,
      send: (f) => socket.send(f),
      onEvent: (e) => this.onSessionEvent(gen, e),
    });
    this.session = session;
    socket.onopen = () => {
      if (gen !== this.generation) return;
      this.opened = true;
      session.start();
    };
    socket.onmessage = (ev) => {
      if (gen !== this.generation) return;
      this.lastRx = this.deps.timers.now();
      if (ev.data instanceof ArrayBuffer) session.receive(new Uint8Array(ev.data));
      else session.receive(new Uint8Array(0)); // text frames are protocol violations
    };
    socket.onclose = () => this.handleClose(gen);
    socket.onerror = () => undefined; // a close always follows
    this.lastRx = this.deps.timers.now();
  }

  private onSessionEvent(gen: number, e: SessionEvent): void {
    if (gen !== this.generation) return;
    if (e.kind === 'ready') {
      this.lastRx = this.deps.timers.now();
      this.handshakeFailures = 0;
      this.set({ phase: 'connected', reason: undefined, attempt: 0, needsRescan: false });
      // The host counts a phone as connected after its first valid encrypted message, so send one now.
      this.send({ type: 'ping', t: this.deps.timers.now() });
      this.schedulePing(gen);
    } else if (e.kind === 'message') {
      const m = e.message;
      if (m.type === 'status') {
        this.set({ status: { platform: m.platform, profile: m.profile, profiles: m.profiles, paused: m.paused } });
      } else if (m.type === 'error') {
        this.hostError = m.code; // the host closes right after
      }
    } else {
      this.closeSocket();
      this.handleClose(gen);
    }
  }

  private schedulePing(gen: number): void {
    this.pingTimer = this.deps.timers.setTimeout(() => {
      if (gen !== this.generation || this.state.phase !== 'connected') return;
      if (this.deps.timers.now() - this.lastRx > SILENCE_LIMIT_MS) {
        // Nothing heard for too long: the network dropped without telling us.
        this.closeSocket();
        this.handleClose(gen);
        return;
      }
      this.send({ type: 'ping', t: this.deps.timers.now() });
      this.schedulePing(gen);
    }, PING_INTERVAL_MS);
  }

  private handleClose(gen: number): void {
    if (gen !== this.generation) return;
    const wasConnected = this.state.phase === 'connected';
    this.teardown();
    if (this.stopped) return;
    const code = this.hostError;
    if (code === 'revoked') {
      this.stopped = true;
      this.set({ phase: 'stopped', reason: 'revoked', needsRescan: true });
    } else if (code === 'replaced' || code === 'idle') {
      this.stopped = true;
      this.set({ phase: 'stopped', reason: code });
    } else if (code === 'paused' || code === 'rate_limited') {
      this.set({ phase: 'waiting', reason: code });
      this.retryAfter(BACKOFF_MAX_MS);
    } else if (!wasConnected && this.opened && ++this.handshakeFailures >= REJECTED_AFTER) {
      // The host answered but the secure handshake never completed (for example its keys changed).
      // Retrying cannot fix that, so offer to scan again right away; keep trying slowly in the background.
      this.set({ phase: 'waiting', reason: 'rejected', attempt: this.state.attempt + 1, needsRescan: true });
      this.retryAfter(BACKOFF_MAX_MS);
    } else {
      const attempt = wasConnected ? 0 : this.state.attempt + 1;
      const needsRescan = attempt >= RESCAN_AFTER_ATTEMPTS;
      this.set({ phase: 'waiting', reason: wasConnected ? 'lost' : 'unreachable', attempt, needsRescan });
      this.retryAfter(backoffDelay(wasConnected ? 0 : attempt - 1));
    }
  }

  private retryAfter(ms: number): void {
    this.clearRetry();
    this.retryTimer = this.deps.timers.setTimeout(() => {
      this.retryTimer = null;
      if (!this.stopped) this.connect();
    }, ms);
  }

  private clearRetry(): void {
    if (this.retryTimer !== null) this.deps.timers.clearTimeout(this.retryTimer);
    this.retryTimer = null;
  }

  private closeSocket(): void {
    const s = this.socket;
    this.socket = null;
    if (s) {
      s.onopen = s.onmessage = s.onclose = s.onerror = null;
      try {
        s.close();
      } catch {
        // already closed
      }
    }
  }

  private teardown(): void {
    if (this.pingTimer !== null) this.deps.timers.clearTimeout(this.pingTimer);
    this.pingTimer = null;
    this.clearRetry();
    this.session?.remoteClosed();
    this.session = null;
    this.closeSocket();
  }
}
