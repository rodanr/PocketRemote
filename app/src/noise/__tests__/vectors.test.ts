import { readFileSync } from 'node:fs';
import path from 'node:path';
import { HandshakeState, KK_PATTERN, XXPSK3_PATTERN, keypairFromPrivate } from '../handshake';
import { fromHex, nodeRng, toHex } from '../../../test-support/util';

const EMPTY = new Uint8Array(0);

interface Vector {
  protocol_name: string;
  init_prologue: string;
  init_static: string;
  init_remote_static?: string;
  init_ephemeral: string;
  init_psks?: string[];
  resp_prologue: string;
  resp_static: string;
  resp_remote_static?: string;
  resp_psks?: string[];
  resp_ephemeral: string;
  handshake_hash: string;
  messages: { payload: string; ciphertext: string }[];
}

function load(file: string): Vector[] {
  const p = path.join(__dirname, '../../../../protocol', file);
  return JSON.parse(readFileSync(p, 'utf8')).vectors;
}

const PATTERNS = {
  [KK_PATTERN.name]: KK_PATTERN,
  [XXPSK3_PATTERN.name]: XXPSK3_PATTERN,
};

function runVector(v: Vector): void {
  const pattern = PATTERNS[v.protocol_name];
  expect(pattern).toBeDefined();
  const init = new HandshakeState({
    pattern,
    role: 'initiator',
    rng: nodeRng,
    s: keypairFromPrivate(fromHex(v.init_static)),
    rs: v.init_remote_static ? fromHex(v.init_remote_static) : undefined,
    psk: v.init_psks?.[0] ? fromHex(v.init_psks[0]) : undefined,
    prologue: fromHex(v.init_prologue),
    ephemeralForTestingOnly: fromHex(v.init_ephemeral),
  });
  const resp = new HandshakeState({
    pattern,
    role: 'responder',
    rng: nodeRng,
    s: keypairFromPrivate(fromHex(v.resp_static)),
    rs: v.resp_remote_static ? fromHex(v.resp_remote_static) : undefined,
    psk: v.resp_psks?.[0] ? fromHex(v.resp_psks[0]) : undefined,
    prologue: fromHex(v.resp_prologue),
    ephemeralForTestingOnly: fromHex(v.resp_ephemeral),
  });

  const hsCount = pattern.messages.length;
  let i = 0;
  for (; i < hsCount; i++) {
    const [sender, receiver] = i % 2 === 0 ? [init, resp] : [resp, init];
    const m = v.messages[i];
    const ct = sender.writeMessage(fromHex(m.payload));
    expect(toHex(ct)).toBe(m.ciphertext);
    expect(toHex(receiver.readMessage(ct))).toBe(m.payload);
  }
  expect(toHex(init.handshakeHash())).toBe(v.handshake_hash);
  expect(toHex(resp.handshakeHash())).toBe(v.handshake_hash);

  const a = init.split();
  const b = resp.split();
  for (; i < v.messages.length; i++) {
    const m = v.messages[i];
    const [sender, receiver] = i % 2 === 0 ? [a.send, b.recv] : [b.send, a.recv];
    const ct = sender.encryptWithAd(EMPTY, fromHex(m.payload));
    expect(toHex(ct)).toBe(m.ciphertext);
    expect(toHex(receiver.decryptWithAd(EMPTY, ct))).toBe(m.payload);
  }
}

describe('public cacophony vectors', () => {
  const vectors = load('noise-cacophony-vectors.json');
  test('both protocol names are covered', () => {
    expect(vectors.map((v) => v.protocol_name).sort()).toEqual([
      KK_PATTERN.name,
      XXPSK3_PATTERN.name,
    ]);
  });
  for (const v of vectors) {
    test(v.protocol_name, () => runVector(v));
  }
});
