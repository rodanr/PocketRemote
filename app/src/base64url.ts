// Strict unpadded base64url decoding (RFC 4648 section 5). Plain TypeScript.
const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';
const LOOKUP = new Map<string, number>([...ALPHABET].map((c, i) => [c, i]));

/** Returns null for anything that is not canonical unpadded base64url. */
export function base64urlDecode(input: string): Uint8Array | null {
  if (input.length % 4 === 1) return null;
  const out = new Uint8Array(Math.floor((input.length * 3) / 4));
  let acc = 0;
  let bits = 0;
  let pos = 0;
  for (const ch of input) {
    const v = LOOKUP.get(ch);
    if (v === undefined) return null;
    acc = (acc << 6) | v;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      out[pos++] = (acc >> bits) & 0xff;
      acc &= (1 << bits) - 1;
    }
  }
  if (acc !== 0) return null; // non-canonical trailing bits
  return out;
}
