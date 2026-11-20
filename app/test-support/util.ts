import { randomBytes } from 'node:crypto';
import type { Rng } from '../src/noise/primitives';

/** Test-only RNG backed by Node. The app uses expo-crypto instead. */
export const nodeRng: Rng = (n) => new Uint8Array(randomBytes(n));

export const fromHex = (h: string): Uint8Array =>
  h.length === 0 ? new Uint8Array(0) : Uint8Array.from(h.match(/../g)!.map((x) => parseInt(x, 16)));

export const toHex = (b: Uint8Array): string =>
  Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
