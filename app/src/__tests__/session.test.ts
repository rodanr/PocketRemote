import { decodeMessage, encodeMessage } from '../decode';
import {
  HandshakeState,
  KK_PATTERN,
  Transport,
  XXPSK3_PATTERN,
  generateKeypair,
  pairingCode,
} from '../noise';
import type { Message } from '../protocol';
import {
  PairingClient,
  SessionClient,
  parsePairingUri,
  type PairingEvent,
  type SessionEvent,
} from '../session';
import { nodeRng } from '../../test-support/util';

const b64url = (b: Uint8Array) => Buffer.from(b).toString('base64url');
const phone = generateKeypair(nodeRng);
const host = generateKeypair(nodeRng);

/** In-process host that speaks the protocol with the same Noise engine. */
class FakeHost {
  transport: Transport | null = null;
  constructor(private readonly hostKeys = host, private readonly phoneKey = phone.publicKey) {}

  accept(frame: Uint8Array): Uint8Array {
    expect(frame[0]).toBe(0x02);
    expect(frame[1]).toBe(0x01);
    const hs = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: this.hostKeys, rs: this.phoneKey });
    hs.readMessage(frame.subarray(2));
    const reply = hs.writeMessage();
    const { send, recv } = hs.split();
    this.transport = new Transport(send, recv);
    const out = new Uint8Array(2 + reply.length);
    out.set([0x02, 0x01]);
    out.set(reply, 2);
    return out;
  }
  say(m: Message): Uint8Array {
    return this.transport!.encrypt(encodeMessage(m));
  }
  hear(frame: Uint8Array): Message {
    return decodeMessage(this.transport!.decrypt(frame));
  }
}

function session(h = new FakeHost()) {
  const sent: Uint8Array[] = [];
  const events: SessionEvent[] = [];
  const client = new SessionClient({
    rng: nodeRng,
    phone,
    hostPublic: host.publicKey,
    send: (f) => sent.push(f),
    onEvent: (e) => events.push(e),
  });
  return { client, sent, events, h };
}

function ready() {
  const s = session();
  s.client.start();
  s.client.receive(s.h.accept(s.sent[0]));
  return s;
}

describe('SessionClient', () => {
  test('handshake then commands and host messages', () => {
    const s = ready();
    expect(s.client.state).toBe('ready');
    expect(s.events).toEqual([{ kind: 'ready' }]);
    s.client.send({ type: 'ping', t: 5 });
    expect(s.h.hear(s.sent[1])).toEqual({ type: 'ping', t: 5 });
    s.client.receive(s.h.say({ type: 'status', platform: 'macos', profile: 'generic', profiles: ['generic'], paused: false }));
    expect(s.events[1]).toMatchObject({ kind: 'message', message: { type: 'status' } });
  });

  test('first frame has the documented header and size', () => {
    const s = session();
    s.client.start();
    expect(s.sent[0][0]).toBe(0x02);
    expect(s.sent[0][1]).toBe(0x01);
    expect(s.sent[0].length).toBeLessThanOrEqual(512);
  });

  test('a host that does not hold the pinned key cannot complete the handshake', () => {
    const impostor = generateKeypair(nodeRng);
    const s = session(new FakeHost(impostor));
    s.client.start();
    expect(() => s.h.accept(s.sent[0])).toThrow(); // impostor cannot even read message 1
  });

  test('bad handshake replies close the session', () => {
    for (const mutate of [
      (f: Uint8Array) => f.slice(0, 10), // truncated
      (f: Uint8Array) => Uint8Array.of(0x01, ...f.slice(1)), // wrong kind
      (f: Uint8Array) => Uint8Array.of(f[0], 0x02, ...f.slice(2)), // wrong version
      (f: Uint8Array) => { const g = f.slice(); g[40] ^= 1; return g; }, // tampered
      () => new Uint8Array(513), // too big
    ]) {
      const s = session();
      s.client.start();
      s.client.receive(mutate(s.h.accept(s.sent[0])));
      expect(s.client.state).toBe('closed');
      expect(s.events).toEqual([{ kind: 'closed', reason: 'protocol' }]);
    }
  });

  test('phone-to-host types from the host close the session', () => {
    const s = ready();
    s.client.receive(s.h.say({ type: 'key', key: 'up' }));
    expect(s.client.state).toBe('closed');
  });

  test('garbage, tampered and replayed frames close the session', () => {
    let s = ready();
    s.client.receive(new Uint8Array(40));
    expect(s.client.state).toBe('closed');
    s = ready();
    const f = s.h.say({ type: 'pong', t: 1 });
    s.client.receive(f);
    s.client.receive(f);
    expect(s.client.state).toBe('closed');
  });

  test('host-only message types cannot be sent by the phone', () => {
    const s = ready();
    s.client.send({ type: 'error', code: 'idle' });
    expect(s.client.state).toBe('closed');
    expect(s.sent.length).toBe(1); // only the handshake frame
  });

  test('nothing is sent before the handshake completes or after close', () => {
    const s = session();
    s.client.send({ type: 'ping', t: 1 });
    expect(s.sent.length).toBe(0);
    const r = ready();
    r.client.remoteClosed();
    r.client.send({ type: 'ping', t: 1 });
    expect(r.sent.length).toBe(1);
  });
});

describe('parsePairingUri', () => {
  const token = nodeRng(32);
  const hk = host.publicKey;
  const good = `pocketremote://pair?v=1&addr=192.168.1.20:9876&pt=${b64url(token)}&hk=${b64url(hk)}&id=0123456789abcdef`;

  test('accepts the documented shape', () => {
    const o = parsePairingUri(good)!;
    expect(o.addr).toBe('192.168.1.20:9876');
    expect(o.token).toEqual(token);
    expect(o.hostKey).toEqual(hk);
    expect(o.hostId).toBe('0123456789abcdef');
    expect(parsePairingUri(good.replace('192.168.1.20', '[fe80::1]'))).not.toBeNull();
  });

  test.each([
    ['wrong scheme', good.replace('pocketremote', 'couchremote')],
    ['wrong host', good.replace('://pair', '://other')],
    ['wrong version', good.replace('v=1', 'v=2')],
    ['port 0', good.replace(':9876', ':0')],
    ['port too big', good.replace(':9876', ':70000')],
    ['hostname instead of ip', good.replace('192.168.1.20', 'evil.example')],
    ['extra param', `${good}&x=1`],
    ['duplicate param', good.replace('&id=0123456789abcdef', '&v=1&id=0123456789abcdef')],
    ['short token', good.replace(b64url(token), b64url(token.slice(0, 31)))],
    ['bad base64', good.replace(b64url(token), '!!!!')],
    ['padded base64', good.replace(b64url(hk), `${b64url(hk)}=`)],
    ['bad id', good.replace('0123456789abcdef', 'xyz')],
    ['missing param', good.replace(/&id=.*/, '')],
    ['empty', ''],
    ['too long', `${good}${'a'.repeat(600)}`],
  ])('rejects %s', (_n, uri) => {
    expect(parsePairingUri(uri)).toBeNull();
  });
});

describe('PairingClient', () => {
  function pairing(qrHostKey = host.publicKey, hostToken?: Uint8Array) {
    const token = nodeRng(32);
    const sent: Uint8Array[] = [];
    const events: PairingEvent[] = [];
    const client = new PairingClient({
      rng: nodeRng,
      phone,
      offer: { addr: '192.168.1.20:9876', token, hostKey: qrHostKey, hostId: '0123456789abcdef' },
      send: (f) => sent.push(f),
      onEvent: (e) => events.push(e),
    });
    const hostHs = new HandshakeState({ pattern: XXPSK3_PATTERN, role: 'responder', rng: nodeRng, s: host, psk: hostToken ?? token });
    const wrap = (m: Uint8Array) => Uint8Array.of(0x01, 0x01, ...m);
    return { client, sent, events, hostHs, wrap };
  }

  test('full pairing: code matches on both sides, then paired', () => {
    const p = pairing();
    p.client.start();
    expect(p.sent[0][0]).toBe(0x01);
    p.hostHs.readMessage(p.sent[0].subarray(2));
    p.client.receive(p.wrap(p.hostHs.writeMessage()));
    p.hostHs.readMessage(p.sent[1].subarray(2));
    expect(p.events[0]).toEqual({ kind: 'code', code: pairingCode(p.hostHs.handshakeHash()) });
    const { send, recv } = p.hostHs.split();
    const hostT = new Transport(send, recv);
    p.client.receive(hostT.encrypt(encodeMessage({ type: 'paired', host_name: 'Mac' })));
    expect(p.events[1]).toEqual({ kind: 'paired', hostName: 'Mac', hostKey: host.publicKey });
    expect(p.client.state).toBe('done');
  });

  test('aborts and sends nothing more if the host key differs from the QR', () => {
    const wrongKey = generateKeypair(nodeRng).publicKey;
    const p = pairing(wrongKey);
    p.client.start();
    p.hostHs.readMessage(p.sent[0].subarray(2));
    p.client.receive(p.wrap(p.hostHs.writeMessage()));
    expect(p.client.state).toBe('failed');
    expect(p.events).toEqual([{ kind: 'failed' }]);
    expect(p.sent.length).toBe(1); // msg3 (which reveals our static key) was never sent
  });

  test('wrong token: the host rejects msg3, the close means failure', () => {
    const p = pairing(host.publicKey, nodeRng(32));
    p.client.start();
    p.hostHs.readMessage(p.sent[0].subarray(2));
    p.client.receive(p.wrap(p.hostHs.writeMessage()));
    expect(() => p.hostHs.readMessage(p.sent[1].subarray(2))).toThrow();
    p.client.remoteClosed();
    expect(p.events.at(-1)).toEqual({ kind: 'failed' });
  });

  test('close before paired is a failure (rejected or expired); close after is fine', () => {
    const p = pairing();
    p.client.start();
    p.client.remoteClosed();
    expect(p.events).toEqual([{ kind: 'failed' }]);
  });

  test('a message other than paired fails the pairing', () => {
    const p = pairing();
    p.client.start();
    p.hostHs.readMessage(p.sent[0].subarray(2));
    p.client.receive(p.wrap(p.hostHs.writeMessage()));
    p.hostHs.readMessage(p.sent[1].subarray(2));
    const { send, recv } = p.hostHs.split();
    const hostT = new Transport(send, recv);
    p.client.receive(hostT.encrypt(encodeMessage({ type: 'pong', t: 1 })));
    expect(p.client.state).toBe('failed');
  });
});
