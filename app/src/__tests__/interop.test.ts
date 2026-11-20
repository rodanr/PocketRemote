// End-to-end: the app's real PairingClient and SessionClient against the real Rust host,
// over a real WebSocket. Skipped unless INTEROP_SERVER_BIN is set: run `npm run test:interop`.
import { spawn, type ChildProcessByStdio } from 'node:child_process';
import type { Readable, Writable } from 'node:stream';
import { createInterface } from 'node:readline';
import WebSocket from 'ws';
import { generateKeypair } from '../noise';
import type { Message } from '../protocol';
import {
  PairingClient,
  SessionClient,
  parsePairingUri,
  type PairingEvent,
  type SessionEvent,
} from '../session';
import { nodeRng } from '../../test-support/util';

const BIN = process.env.INTEROP_SERVER_BIN;
const suite = BIN ? describe : describe.skip;

type Line = { event: string; [k: string]: unknown };

suite('interop with the real Rust host', () => {
  jest.setTimeout(30000);
  let proc: ChildProcessByStdio<Writable, Readable, null>;
  const lines: Line[] = [];
  let addr = '';
  let qr = '';

  const until = async <T>(what: string, f: () => T | undefined, ms = 5000): Promise<T> => {
    const t0 = Date.now();
    for (;;) {
      const v = f();
      if (v !== undefined) return v;
      if (Date.now() - t0 > ms) throw new Error(`timed out waiting for ${what}`);
      await new Promise((r) => setTimeout(r, 10));
    }
  };
  const line = (event: string, from = 0) => lines.slice(from).find((l) => l.event === event);

  beforeAll(async () => {
    proc = spawn(BIN!, [], { stdio: ['pipe', 'pipe', 'inherit'] });
    createInterface({ input: proc.stdout }).on('line', (l) => lines.push(JSON.parse(l)));
    const listening = await until('listening', () => line('listening'));
    addr = listening.addr as string;
    qr = listening.qr as string;
  });

  afterAll(() => {
    proc.stdin.write('quit\n');
    proc.kill();
  });

  function connect(onOpen: () => void, onFrame: (f: Uint8Array) => void, onClose: () => void): WebSocket {
    const ws = new WebSocket(`ws://${addr}/ws`);
    ws.binaryType = 'nodebuffer';
    ws.on('open', onOpen);
    ws.on('message', (d: Buffer) => onFrame(new Uint8Array(d)));
    ws.on('close', onClose);
    ws.on('error', () => {});
    return ws;
  }

  function pair(offerUri: string, phone = generateKeypair(nodeRng)) {
    const offer = parsePairingUri(offerUri)!;
    const events: PairingEvent[] = [];
    let ws: WebSocket;
    const client: PairingClient = new PairingClient({
      rng: nodeRng,
      phone,
      offer,
      send: (f) => ws.send(f),
      onEvent: (e) => events.push(e),
    });
    ws = connect(() => client.start(), (f) => client.receive(f), () => client.remoteClosed());
    return { client, events, ws, phone, offer };
  }

  function openSession(phone: ReturnType<typeof generateKeypair>, hostKey: Uint8Array) {
    const events: SessionEvent[] = [];
    let ws: WebSocket;
    const client: SessionClient = new SessionClient({
      rng: nodeRng,
      phone,
      hostPublic: hostKey,
      send: (f) => ws.send(f),
      onEvent: (e) => events.push(e),
    });
    ws = connect(() => client.start(), (f) => client.receive(f), () => client.remoteClosed());
    return { client, events, ws };
  }

  test('QR from the host parses, pairing needs host approval, then a session works', async () => {
    const p = pair(qr);
    const code = await until('code event', () => p.events.find((e) => e.kind === 'code'));
    const approval = await until('approval line', () => line('approval'));
    expect(code).toEqual({ kind: 'code', code: approval.code });
    expect(line('approve_result')).toBeUndefined();

    proc.stdin.write('approve\n');
    const paired = await until('paired event', () => p.events.find((e) => e.kind === 'paired'));
    expect(paired).toMatchObject({ kind: 'paired', hostName: 'Interop Mac' });
    await until('approve ok', () => line('approve_result'));
    expect(line('approve_result')).toMatchObject({ ok: true });

    // the QR is single use
    const again = pair(qr);
    await until('second pairing fails', () => again.events.find((e) => e.kind === 'failed'));

    // a session with the paired phone key and the pinned host key
    const s = openSession(p.phone, p.offer.hostKey);
    await until('ready', () => s.events.find((e) => e.kind === 'ready'));
    const status = await until('status', () => s.events.find((e) => e.kind === 'message'));
    expect(status).toMatchObject({ kind: 'message', message: { type: 'status', platform: 'macos' } });

    s.client.send({ type: 'ping', t: 7 });
    await until('pong', () =>
      s.events.find((e) => e.kind === 'message' && e.message.type === 'pong' && e.message.t === 7),
    );
    const cmds: Message[] = [
      { type: 'move', dx: 3.5, dy: -2 },
      { type: 'button', button: 'left', state: 'down' },
      { type: 'text', value: 'héllo ✓', submit: true },
      { type: 'media', action: 'vol_up' },
    ];
    cmds.forEach((m) => s.client.send(m));
    await until('commands arrive', () => (lines.filter((l) => l.event === 'cmd').length >= 4 ? true : undefined));
    expect(lines.filter((l) => l.event === 'cmd').map((l) => l.msg)).toEqual(cmds);

    const before = lines.length;
    s.ws.close();
    await until('held input released', () => line('released', before));

    // a phone that was never paired cannot open a session
    const stranger = openSession(generateKeypair(nodeRng), p.offer.hostKey);
    await until('stranger closed', () => (stranger.client.state === 'closed' ? true : undefined));
    expect(stranger.events.some((e) => e.kind === 'ready')).toBe(false);
  });
});
