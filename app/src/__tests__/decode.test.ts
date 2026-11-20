import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { DecodeError, decodeMessage, encodeMessage, isHostToPhone, isPhoneToHost } from '../decode';
import { MAX_PLAINTEXT_BYTES } from '../protocol';

const FIXTURES = path.join(__dirname, '../../../protocol/fixtures');
const load = (kind: 'valid' | 'invalid') =>
  readdirSync(path.join(FIXTURES, kind))
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((f) => [f, new Uint8Array(readFileSync(path.join(FIXTURES, kind, f)))] as const);

const enc = (o: unknown) => new TextEncoder().encode(JSON.stringify(o));

describe('shared fixtures', () => {
  const valid = load('valid');
  const invalid = load('invalid');

  test('there are enough fixtures and every message type has a valid one', () => {
    expect(valid.length).toBeGreaterThanOrEqual(15);
    expect(invalid.length).toBeGreaterThanOrEqual(12);
    const types = new Set(valid.map(([, b]) => JSON.parse(new TextDecoder().decode(b)).type));
    for (const t of ['paired', 'status', 'ping', 'pong', 'move', 'click', 'button', 'scroll', 'key', 'media', 'player', 'browser', 'text', 'profile', 'error']) {
      expect(types).toContain(t);
    }
  });

  test.each(valid)('valid %s decodes and round-trips', (_name, bytes) => {
    const msg = decodeMessage(bytes);
    expect(JSON.parse(new TextDecoder().decode(encodeMessage(msg)))).toEqual(
      JSON.parse(new TextDecoder().decode(bytes)),
    );
  });

  test.each(invalid)('invalid %s is rejected', (_name, bytes) => {
    expect(() => decodeMessage(bytes)).toThrow(DecodeError);
  });
});

describe('limits', () => {
  test('oversized frame', () => {
    expect(() => decodeMessage(new Uint8Array(MAX_PLAINTEXT_BYTES + 1).fill(32))).toThrow(DecodeError);
  });

  test('text limit counts scalar values, not UTF-16 units or bytes', () => {
    expect(() => decodeMessage(enc({ type: 'text', value: 'é'.repeat(500), submit: false }))).not.toThrow();
    expect(() => decodeMessage(enc({ type: 'text', value: 'é'.repeat(501), submit: false }))).toThrow(DecodeError);
    // an emoji is one scalar value but two UTF-16 units
    expect(() => decodeMessage(enc({ type: 'text', value: '😀'.repeat(500), submit: false }))).not.toThrow();
    expect(() => decodeMessage(enc({ type: 'text', value: '😀'.repeat(501), submit: false }))).toThrow(DecodeError);
  });

  test('invalid UTF-8 is rejected', () => {
    expect(() => decodeMessage(Uint8Array.of(0x7b, 0xff, 0xfe, 0x7d))).toThrow(DecodeError);
  });

  test('extra and missing fields, wrong types, prototype tricks', () => {
    expect(() => decodeMessage(enc({ type: 'key', key: 'up', extra: 1 }))).toThrow(DecodeError);
    expect(() => decodeMessage(enc({ type: 'key' }))).toThrow(DecodeError);
    expect(() => decodeMessage(enc({ type: 'constructor' }))).toThrow(DecodeError);
    expect(() => decodeMessage(enc({ type: '__proto__' }))).toThrow(DecodeError);
    expect(() => decodeMessage(enc({ type: 'move', dx: 1, dy: NaN }))).toThrow(DecodeError);
    expect(() => decodeMessage(enc({ type: 'move', dx: 10001, dy: 0 }))).toThrow(DecodeError);
    expect(() => decodeMessage(enc({ type: 'ping', t: 1.5 }))).toThrow(DecodeError);
  });
});

describe('directions', () => {
  test('phone to host and host to phone sets', () => {
    expect(isPhoneToHost(decodeMessage(enc({ type: 'key', key: 'up' })))).toBe(true);
    expect(isHostToPhone(decodeMessage(enc({ type: 'key', key: 'up' })))).toBe(false);
    const err = decodeMessage(enc({ type: 'error', code: 'idle' }));
    expect(isPhoneToHost(err)).toBe(false);
    expect(isHostToPhone(err)).toBe(true);
    const ping = decodeMessage(enc({ type: 'ping', t: 1 }));
    expect(isPhoneToHost(ping) && isHostToPhone(ping)).toBe(true);
  });
});
