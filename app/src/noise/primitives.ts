// Noise primitives (spec rev 34): ChaChaPoly, SHA256, HKDF over HMAC-SHA256, CipherState.
// Plain TypeScript: no React Native imports. Never log keys, tokens or payloads.
import { chacha20poly1305 } from '@noble/ciphers/chacha.js';
import { hmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';

export const DHLEN = 32;
export const HASHLEN = 32;
export const TAGLEN = 16;
const MAX_NONCE = (1n << 64n) - 1n;

/** Opaque failure. Carries no detail on purpose: callers close the socket. */
export class NoiseError extends Error {
  constructor() {
    super('noise failure');
    this.name = 'NoiseError';
  }
}

/** Source of cryptographically secure random bytes (expo-crypto in the app, node:crypto in tests). */
export type Rng = (length: number) => Uint8Array;

export function concat(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let i = 0;
  for (const p of parts) {
    out.set(p, i);
    i += p.length;
  }
  return out;
}

/** Constant-time comparison for equal-length arrays (false if lengths differ). */
export function equalBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}

export function hash(data: Uint8Array): Uint8Array {
  return sha256(data);
}

/** HKDF from the Noise spec: 2 or 3 outputs. */
export function hkdf(chainingKey: Uint8Array, ikm: Uint8Array, outputs: 2 | 3): Uint8Array[] {
  const temp = hmac(sha256, chainingKey, ikm);
  const o1 = hmac(sha256, temp, Uint8Array.of(1));
  const o2 = hmac(sha256, temp, concat(o1, Uint8Array.of(2)));
  if (outputs === 2) return [o1, o2];
  const o3 = hmac(sha256, temp, concat(o2, Uint8Array.of(3)));
  return [o1, o2, o3];
}

function nonceBytes(n: bigint): Uint8Array {
  const b = new Uint8Array(12); // 4 zero bytes, then 64-bit little-endian counter
  new DataView(b.buffer).setBigUint64(4, n, true);
  return b;
}

export class CipherState {
  private k: Uint8Array | null = null;
  private n = 0n;

  initializeKey(key: Uint8Array | null): void {
    this.k = key;
    this.n = 0n;
  }

  hasKey(): boolean {
    return this.k !== null;
  }

  encryptWithAd(ad: Uint8Array, plaintext: Uint8Array): Uint8Array {
    if (this.k === null) return plaintext;
    if (this.n === MAX_NONCE) throw new NoiseError();
    const out = chacha20poly1305(this.k, nonceBytes(this.n), ad).encrypt(plaintext);
    this.n += 1n;
    return out;
  }

  decryptWithAd(ad: Uint8Array, ciphertext: Uint8Array): Uint8Array {
    if (this.k === null) return ciphertext;
    if (this.n === MAX_NONCE) throw new NoiseError();
    let out: Uint8Array;
    try {
      out = chacha20poly1305(this.k, nonceBytes(this.n), ad).decrypt(ciphertext);
    } catch {
      throw new NoiseError();
    }
    this.n += 1n;
    return out;
  }

  /** REKEY: first 32 bytes of ENCRYPT(k, maxnonce, "", 32 zero bytes). Nonce keeps counting. */
  rekey(): void {
    if (this.k === null) throw new NoiseError();
    const out = chacha20poly1305(this.k, nonceBytes(MAX_NONCE), new Uint8Array(0)).encrypt(
      new Uint8Array(32),
    );
    this.k.fill(0);
    this.k = out.slice(0, 32);
  }
}
