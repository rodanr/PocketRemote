import { HandshakeState, KK_PATTERN, XXPSK3_PATTERN, generateKeypair } from '../handshake';
import { NoiseError } from '../primitives';
import { MAX_TRANSPORT_FRAME, Transport } from '../transport';
import { pairingCode } from '../index';
import { nodeRng } from '../../../test-support/util';

const phone = generateKeypair(nodeRng);
const host = generateKeypair(nodeRng);

function kk(): { phoneT: Transport; hostT: Transport } {
  const i = new HandshakeState({ pattern: KK_PATTERN, role: 'initiator', rng: nodeRng, s: phone, rs: host.publicKey });
  const r = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: host, rs: phone.publicKey });
  r.readMessage(i.writeMessage());
  i.readMessage(r.writeMessage());
  const a = i.split();
  const b = r.split();
  return { phoneT: new Transport(a.send, a.recv), hostT: new Transport(b.send, b.recv) };
}

describe('session (KK)', () => {
  test('round trip in both directions', () => {
    const { phoneT, hostT } = kk();
    const text = new TextEncoder().encode('hello');
    expect(hostT.decrypt(phoneT.encrypt(text))).toEqual(text);
    expect(phoneT.decrypt(hostT.encrypt(text))).toEqual(text);
  });

  test('wrong pinned host key fails', () => {
    const other = generateKeypair(nodeRng);
    const i = new HandshakeState({ pattern: KK_PATTERN, role: 'initiator', rng: nodeRng, s: phone, rs: other.publicKey });
    const r = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: host, rs: phone.publicKey });
    expect(() => r.readMessage(i.writeMessage())).toThrow(NoiseError);
  });

  test('tampered, replayed, reordered and oversized frames are rejected', () => {
    const { phoneT, hostT } = kk();
    const f1 = phoneT.encrypt(Uint8Array.of(1));
    const f2 = phoneT.encrypt(Uint8Array.of(2));
    expect(() => hostT.decrypt(f2)).toThrow(NoiseError); // out of order
    const tampered = f1.slice();
    tampered[0] ^= 1;
    expect(() => hostT.decrypt(tampered)).toThrow(NoiseError);
    expect(hostT.decrypt(f1)).toEqual(Uint8Array.of(1));
    expect(() => hostT.decrypt(f1)).toThrow(NoiseError); // replay
    expect(() => hostT.decrypt(new Uint8Array(MAX_TRANSPORT_FRAME + 1))).toThrow(NoiseError);
    expect(() => hostT.decrypt(new Uint8Array(3))).toThrow(NoiseError);
    expect(() => phoneT.encrypt(new Uint8Array(MAX_TRANSPORT_FRAME))).toThrow(NoiseError);
  });

  test('handshake steps must be in order', () => {
    const r = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: host, rs: phone.publicKey });
    expect(() => r.writeMessage()).toThrow(NoiseError);
    expect(() => r.readMessage(new Uint8Array(10))).toThrow(NoiseError);
  });
});

describe('pairing (XXpsk3)', () => {
  function run(phoneToken: Uint8Array, hostToken: Uint8Array) {
    const i = new HandshakeState({ pattern: XXPSK3_PATTERN, role: 'initiator', rng: nodeRng, s: phone, psk: phoneToken });
    const r = new HandshakeState({ pattern: XXPSK3_PATTERN, role: 'responder', rng: nodeRng, s: host, psk: hostToken });
    r.readMessage(i.writeMessage());
    i.readMessage(r.writeMessage());
    const hostKeySeenByPhone = i.remoteStatic();
    r.readMessage(i.writeMessage());
    return { i, r, hostKeySeenByPhone };
  }

  test('both sides get the same code and see each other keys', () => {
    const token = nodeRng(32);
    const { i, r, hostKeySeenByPhone } = run(token, token);
    expect(hostKeySeenByPhone).toEqual(host.publicKey);
    expect(r.remoteStatic()).toEqual(phone.publicKey);
    const code = pairingCode(i.handshakeHash());
    expect(code).toBe(pairingCode(r.handshakeHash()));
    expect(code).toMatch(/^\d{6}$/);
  });

  test('wrong token fails on the third message', () => {
    expect(() => run(nodeRng(32), nodeRng(32))).toThrow(NoiseError);
  });

  test('rejects a token that is not 32 bytes', () => {
    expect(
      () => new HandshakeState({ pattern: XXPSK3_PATTERN, role: 'initiator', rng: nodeRng, s: phone, psk: new Uint8Array(31) }),
    ).toThrow(NoiseError);
  });

  test('code formula matches the spec', () => {
    expect(pairingCode(Uint8Array.of(0, 0, 0, 0, 9, 9))).toBe('000000');
    expect(pairingCode(Uint8Array.of(0, 0, 0, 1))).toBe('000001');
    expect(pairingCode(Uint8Array.of(0x00, 0x0f, 0x42, 0x40))).toBe('000000');
    expect(pairingCode(Uint8Array.of(0xff, 0xff, 0xff, 0xff))).toBe('967295');
  });
});

describe('rekey', () => {
  test('both sides stay in step across the interval', () => {
    const i = new HandshakeState({ pattern: KK_PATTERN, role: 'initiator', rng: nodeRng, s: phone, rs: host.publicKey });
    const r = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: host, rs: phone.publicKey });
    r.readMessage(i.writeMessage());
    i.readMessage(r.writeMessage());
    const a = i.split();
    const b = r.split();
    const phoneT = new Transport(a.send, a.recv, 3);
    const hostT = new Transport(b.send, b.recv, 3);
    for (let n = 0; n < 20; n++) {
      expect(hostT.decrypt(phoneT.encrypt(Uint8Array.of(n)))).toEqual(Uint8Array.of(n));
      expect(phoneT.decrypt(hostT.encrypt(Uint8Array.of(n, n)))).toEqual(Uint8Array.of(n, n));
    }
  });

  test('a side that does not rekey breaks the session', () => {
    const i = new HandshakeState({ pattern: KK_PATTERN, role: 'initiator', rng: nodeRng, s: phone, rs: host.publicKey });
    const r = new HandshakeState({ pattern: KK_PATTERN, role: 'responder', rng: nodeRng, s: host, rs: phone.publicKey });
    r.readMessage(i.writeMessage());
    i.readMessage(r.writeMessage());
    const a = i.split();
    const b = r.split();
    const phoneT = new Transport(a.send, a.recv, 2);
    const hostT = new Transport(b.send, b.recv); // default interval: never rekeys here
    expect(hostT.decrypt(phoneT.encrypt(Uint8Array.of(1)))).toEqual(Uint8Array.of(1));
    expect(hostT.decrypt(phoneT.encrypt(Uint8Array.of(2)))).toEqual(Uint8Array.of(2));
    expect(() => hostT.decrypt(phoneT.encrypt(Uint8Array.of(3)))).toThrow(NoiseError);
  });
});
