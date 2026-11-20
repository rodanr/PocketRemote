import { backoffDelay, RemoteConnection, type SocketLike, type ConnState } from '../connection';
import { decodeMessage, encodeMessage } from '../decode';
import { HandshakeState, KK_PATTERN, Transport, generateKeypair } from '../noise';
import type { ErrorCode, Message } from '../protocol';
import { nodeRng } from '../../test-support/util';

const phone = generateKeypair(nodeRng);
const host = generateKeypair(nodeRng);

class FakeSocket implements SocketLike {
  binaryType = 'blob';
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  sent: Uint8Array[] = [];
  closed = false;
  transport: Transport | null = null;
  constructor(readonly url: string) {}
  send(d: Uint8Array) {
    this.sent.push(d);
  }
  close() {
    this.closed = true;
  }
  /** Host side: answer the client's first frame and complete the handshake. */
  acceptHandshake() {
    const hs = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: host, rs: phone.publicKey });
    hs.readMessage(this.sent[0].subarray(2));
    const reply = hs.writeMessage();
    const { send, recv } = hs.split();
    this.transport = new Transport(send, recv);
    this.deliver(Uint8Array.of(0x02, 0x01, ...reply));
  }
  deliver(bytes: Uint8Array) {
    const ab = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
    this.onmessage?.({ data: ab });
  }
  say(m: Message) {
    this.deliver(this.transport!.encrypt(encodeMessage(m)));
  }
  hear(i: number): Message {
    return decodeMessage(this.transport!.decrypt(this.sent[i]));
  }
  open() {
    this.onopen?.();
  }
  drop() {
    this.onclose?.({});
  }
}

function rig() {
  let now = 1000;
  const timers: { at: number; fn: () => void; live: boolean }[] = [];
  const sockets: FakeSocket[] = [];
  const states: ConnState[] = [];
  const conn = new RemoteConnection(
    {
      createSocket: (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      rng: nodeRng,
      timers: {
        setTimeout: (fn, ms) => {
          const t = { at: now + ms, fn, live: true };
          timers.push(t);
          return t;
        },
        clearTimeout: (h) => {
          (h as { live: boolean }).live = false;
        },
        now: () => now,
      },
    },
    { addr: '192.168.1.20:9876', phone, hostKey: host.publicKey },
  );
  conn.subscribe((s) => states.push(s));
  const advance = (ms: number) => {
    const end = now + ms;
    for (;;) {
      const next = timers.filter((t) => t.live && t.at <= end).sort((a, b) => a.at - b.at)[0];
      if (!next) break;
      now = next.at;
      next.live = false;
      next.fn();
    }
    now = end;
  };
  const connectUp = () => {
    conn.start();
    const s = sockets.at(-1)!;
    s.open();
    s.acceptHandshake();
    s.say({ type: 'status', platform: 'macos', profile: 'generic', profiles: ['generic'], paused: false });
    return s;
  };
  return { conn, sockets, states, advance, connectUp, now: () => now };
}

describe('backoff', () => {
  test('250 ms doubling up to 5 s', () => {
    expect([0, 1, 2, 3, 4, 5, 6, 10].map(backoffDelay)).toEqual([250, 500, 1000, 2000, 4000, 5000, 5000, 5000]);
  });
});

describe('RemoteConnection', () => {
  test('connects to the stored address, completes the handshake and exposes status', () => {
    const r = rig();
    const s = r.connectUp();
    expect(s.url).toBe('ws://192.168.1.20:9876/ws');
    expect(s.binaryType).toBe('arraybuffer');
    expect(r.conn.getState()).toMatchObject({ phase: 'connected', attempt: 0, status: { platform: 'macos' } });
  });

  test('send works only while connected', () => {
    const r = rig();
    expect(r.conn.send({ type: 'key', key: 'up' })).toBe(false);
    const s = r.connectUp();
    expect(r.conn.send({ type: 'key', key: 'up' })).toBe(true);
    expect(s.hear(1)).toEqual({ type: 'key', key: 'up' });
    s.drop();
    expect(r.conn.send({ type: 'key', key: 'up' })).toBe(false);
  });

  test('pings every 5 seconds', () => {
    const r = rig();
    const s = r.connectUp();
    r.advance(5000);
    expect(s.hear(1)).toMatchObject({ type: 'ping' });
    s.say({ type: 'pong', t: 1 });
    r.advance(5000);
    expect(s.hear(2)).toMatchObject({ type: 'ping' });
  });

  test('a silent network is noticed after 15 s and the connection is retried', () => {
    const r = rig();
    const s = r.connectUp();
    r.advance(20000); // no pong ever arrives
    expect(s.closed).toBe(true);
    expect(r.conn.getState()).toMatchObject({ phase: 'waiting', reason: 'lost' });
    r.advance(300);
    expect(r.sockets.length).toBe(2);
  });

  test('failed attempts back off 250, 500, 1000 ... and offer a re-scan after six', () => {
    const r = rig();
    r.conn.start();
    const delays: number[] = [];
    for (let i = 0; i < 6; i++) {
      const s = r.sockets.at(-1)!;
      const t0 = r.now();
      s.drop();
      const before = r.sockets.length;
      let waited = 0;
      while (r.sockets.length === before && waited < 6000) {
        r.advance(10);
        waited += 10;
      }
      delays.push(r.now() - t0);
    }
    expect(delays).toEqual([250, 500, 1000, 2000, 4000, 5000]);
    expect(r.conn.getState()).toMatchObject({ needsRescan: true, attempt: 6 });
  });

  test('a good connection resets the backoff', () => {
    const r = rig();
    r.conn.start();
    r.sockets[0].drop();
    r.advance(250);
    const s = r.sockets[1];
    s.open();
    s.acceptHandshake();
    s.say({ type: 'status', platform: 'macos', profile: 'generic', profiles: ['generic'], paused: false });
    expect(r.conn.getState().attempt).toBe(0);
    s.drop();
    r.advance(249);
    expect(r.sockets.length).toBe(2);
    r.advance(2);
    expect(r.sockets.length).toBe(3);
  });

  test.each<[ErrorCode, string, boolean]>([
    ['revoked', 'stopped', false],
    ['replaced', 'stopped', false],
    ['idle', 'stopped', false],
  ])('host error %s ends the session without retrying', (code, phase, retries) => {
    const r = rig();
    const s = r.connectUp();
    s.say({ type: 'error', code });
    s.drop();
    expect(r.conn.getState()).toMatchObject({ phase, reason: code });
    r.advance(60000);
    expect(r.sockets.length > 1).toBe(retries);
  });

  test('revoked asks for a new pairing', () => {
    const r = rig();
    const s = r.connectUp();
    s.say({ type: 'error', code: 'revoked' });
    s.drop();
    expect(r.conn.getState().needsRescan).toBe(true);
  });

  test('paused and rate_limited retry every 5 seconds', () => {
    for (const code of ['paused', 'rate_limited'] as const) {
      const r = rig();
      const s = r.connectUp();
      s.say({ type: 'error', code });
      s.drop();
      expect(r.conn.getState()).toMatchObject({ phase: 'waiting', reason: code });
      r.advance(4999);
      expect(r.sockets.length).toBe(1);
      r.advance(2);
      expect(r.sockets.length).toBe(2);
    }
  });

  test('reconnectNow retries immediately after a stop, and does nothing while connected', () => {
    const r = rig();
    const s = r.connectUp();
    r.conn.reconnectNow();
    expect(r.sockets.length).toBe(1);
    s.say({ type: 'error', code: 'idle' });
    s.drop();
    r.conn.reconnectNow();
    expect(r.sockets.length).toBe(2);
    expect(r.conn.getState().phase).toBe('connecting');
  });

  test('stop closes the socket and stays stopped', () => {
    const r = rig();
    const s = r.connectUp();
    r.conn.stop();
    expect(s.closed).toBe(true);
    expect(r.conn.getState().phase).toBe('stopped');
    r.advance(60000);
    expect(r.sockets.length).toBe(1);
  });

  test('a bad handshake reply (wrong host) closes the socket and backs off', () => {
    const r = rig();
    r.conn.start();
    const s = r.sockets[0];
    s.open();
    s.deliver(new Uint8Array(40)); // garbage instead of the host's reply
    expect(s.closed).toBe(true);
    expect(r.conn.getState()).toMatchObject({ phase: 'waiting', reason: 'unreachable' });
  });

  test('frames from an old socket are ignored after a reconnect', () => {
    const r = rig();
    r.conn.start();
    const old = r.sockets[0];
    old.drop();
    r.advance(250);
    expect(r.sockets.length).toBe(2);
    old.deliver(new Uint8Array(10));
    expect(r.conn.getState().phase).toBe('connecting');
  });
});
