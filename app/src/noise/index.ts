export { NoiseError, equalBytes, type Rng } from './primitives';
export {
  HandshakeState,
  KK_PATTERN,
  XXPSK3_PATTERN,
  generateKeypair,
  keypairFromPrivate,
  type HandshakeOptions,
  type Keypair,
  type Pattern,
  type Role,
} from './handshake';
export { MAX_TRANSPORT_FRAME, REKEY_INTERVAL, Transport } from './transport';

/** Handshake frames are `[kind][version][noise message]`, at most 512 bytes (PLAN.md §4.1). */
export const HANDSHAKE_VERSION = 0x01;
export const MAX_HANDSHAKE_FRAME = 512;
export const KIND_PAIRING = 0x01;
export const KIND_SESSION = 0x02;

/** Six digits from the first 4 bytes of the handshake hash, big-endian, mod 1,000,000. */
export function pairingCode(handshakeHash: Uint8Array): string {
  const n = new DataView(handshakeHash.buffer, handshakeHash.byteOffset, 4).getUint32(0, false);
  return (n % 1_000_000).toString().padStart(6, '0');
}
