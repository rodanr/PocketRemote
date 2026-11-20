// Generic Noise handshake engine for the two patterns PocketRemote uses:
//   Noise_KK_25519_ChaChaPoly_SHA256       (sessions)
//   Noise_XXpsk3_25519_ChaChaPoly_SHA256   (pairing)
// Follows the Noise specification (rev 34). Do not add other patterns without asking.
import { x25519 } from '@noble/curves/ed25519.js';
import {
  CipherState,
  DHLEN,
  HASHLEN,
  NoiseError,
  TAGLEN,
  concat,
  equalBytes,
  hash,
  hkdf,
  type Rng,
} from './primitives';

export type Token = 'e' | 's' | 'ee' | 'es' | 'se' | 'ss' | 'psk';

export interface Pattern {
  readonly name: string;
  readonly hasPsk: boolean;
  /** Static keys known to both sides before the handshake (`-> s` / `<- s`). */
  readonly preInitiatorStatic: boolean;
  readonly preResponderStatic: boolean;
  readonly messages: readonly (readonly Token[])[];
}

export const KK_PATTERN: Pattern = {
  name: 'Noise_KK_25519_ChaChaPoly_SHA256',
  hasPsk: false,
  preInitiatorStatic: true,
  preResponderStatic: true,
  messages: [
    ['e', 'es', 'ss'],
    ['e', 'ee', 'se'],
  ],
};

export const XXPSK3_PATTERN: Pattern = {
  name: 'Noise_XXpsk3_25519_ChaChaPoly_SHA256',
  hasPsk: true,
  preInitiatorStatic: false,
  preResponderStatic: false,
  messages: [['e'], ['e', 'ee', 's', 'es'], ['s', 'se', 'psk']],
};

export interface Keypair {
  privateKey: Uint8Array;
  publicKey: Uint8Array;
}

export type Role = 'initiator' | 'responder';

export interface HandshakeOptions {
  pattern: Pattern;
  role: Role;
  rng: Rng;
  /** Our static keypair. */
  s?: Keypair;
  /** The peer's static public key, if known in advance (KK). */
  rs?: Uint8Array;
  /** 32-byte pre-shared key (pairing token) for psk patterns. */
  psk?: Uint8Array;
  prologue?: Uint8Array;
  /** Fixed ephemeral private key. FOR TEST VECTORS ONLY. */
  ephemeralForTestingOnly?: Uint8Array;
}

export function keypairFromPrivate(privateKey: Uint8Array): Keypair {
  return { privateKey, publicKey: x25519.getPublicKey(privateKey) };
}

export function generateKeypair(rng: Rng): Keypair {
  return keypairFromPrivate(rng(32));
}

function dh(priv: Uint8Array, pub: Uint8Array): Uint8Array {
  try {
    return x25519.getSharedSecret(priv, pub);
  } catch {
    throw new NoiseError();
  }
}

const MAX_MESSAGE = 65535;
const EMPTY = new Uint8Array(0);

export class HandshakeState {
  private readonly pattern: Pattern;
  private readonly role: Role;
  private readonly rng: Rng;
  private readonly s: Keypair | undefined;
  private rs: Uint8Array | undefined;
  private e: Keypair | undefined;
  private re: Uint8Array | undefined;
  private readonly psk: Uint8Array | undefined;
  private readonly fixedE: Uint8Array | undefined;
  private index = 0;
  private ck: Uint8Array;
  private h: Uint8Array;
  private readonly cipher = new CipherState();

  constructor(o: HandshakeOptions) {
    this.pattern = o.pattern;
    this.role = o.role;
    this.rng = o.rng;
    this.s = o.s;
    this.rs = o.rs;
    this.psk = o.psk;
    this.fixedE = o.ephemeralForTestingOnly;

    if (o.pattern.hasPsk && (!o.psk || o.psk.length !== 32)) throw new NoiseError();
    const needsS =
      o.pattern.preInitiatorStatic || o.pattern.preResponderStatic || o.pattern.messages.flat().includes('s');
    if (needsS && !o.s) throw new NoiseError();
    const needsRs = o.role === 'initiator' ? o.pattern.preResponderStatic : o.pattern.preInitiatorStatic;
    if (needsRs && (!o.rs || o.rs.length !== DHLEN)) throw new NoiseError();

    const name = new TextEncoder().encode(o.pattern.name);
    if (name.length <= HASHLEN) {
      this.h = new Uint8Array(HASHLEN);
      this.h.set(name);
    } else {
      this.h = hash(name);
    }
    this.ck = this.h;
    this.mixHash(o.prologue ?? EMPTY);
    if (o.pattern.preInitiatorStatic) {
      this.mixHash(o.role === 'initiator' ? o.s!.publicKey : o.rs!);
    }
    if (o.pattern.preResponderStatic) {
      this.mixHash(o.role === 'responder' ? o.s!.publicKey : o.rs!);
    }
  }

  private mixHash(data: Uint8Array): void {
    this.h = hash(concat(this.h, data));
  }

  private mixKey(ikm: Uint8Array): void {
    const [ck, k] = hkdf(this.ck, ikm, 2);
    this.ck = ck;
    this.cipher.initializeKey(k);
  }

  private mixKeyAndHash(ikm: Uint8Array): void {
    const [ck, tempH, tempK] = hkdf(this.ck, ikm, 3);
    this.ck = ck;
    this.mixHash(tempH);
    this.cipher.initializeKey(tempK);
  }

  private encryptAndHash(plaintext: Uint8Array): Uint8Array {
    const ct = this.cipher.encryptWithAd(this.h, plaintext);
    this.mixHash(ct);
    return ct;
  }

  private decryptAndHash(ciphertext: Uint8Array): Uint8Array {
    const pt = this.cipher.decryptWithAd(this.h, ciphertext);
    this.mixHash(ciphertext);
    return pt;
  }

  private get isInitiator(): boolean {
    return this.role === 'initiator';
  }

  private myTurn(): boolean {
    return (this.index % 2 === 0) === this.isInitiator;
  }

  isFinished(): boolean {
    return this.index >= this.pattern.messages.length;
  }

  private dhToken(token: 'ee' | 'es' | 'se' | 'ss'): Uint8Array {
    const e = this.e;
    const s = this.s;
    const re = this.re;
    const rs = this.rs;
    const need = <T>(v: T | undefined): T => {
      if (v === undefined) throw new NoiseError();
      return v;
    };
    switch (token) {
      case 'ee':
        return dh(need(e).privateKey, need(re));
      case 'ss':
        return dh(need(s).privateKey, need(rs));
      case 'es': // initiator ephemeral with responder static
        return this.isInitiator ? dh(need(e).privateKey, need(rs)) : dh(need(s).privateKey, need(re));
      case 'se': // initiator static with responder ephemeral
        return this.isInitiator ? dh(need(s).privateKey, need(re)) : dh(need(e).privateKey, need(rs));
    }
  }

  writeMessage(payload: Uint8Array = EMPTY): Uint8Array {
    if (this.isFinished() || !this.myTurn()) throw new NoiseError();
    const parts: Uint8Array[] = [];
    for (const token of this.pattern.messages[this.index]) {
      switch (token) {
        case 'e': {
          this.e = this.fixedE
            ? keypairFromPrivate(this.fixedE)
            : generateKeypair(this.rng);
          parts.push(this.e.publicKey);
          this.mixHash(this.e.publicKey);
          if (this.pattern.hasPsk) this.mixKey(this.e.publicKey);
          break;
        }
        case 's':
          parts.push(this.encryptAndHash(this.s!.publicKey));
          break;
        case 'psk':
          this.mixKeyAndHash(this.psk!);
          break;
        default:
          this.mixKey(this.dhToken(token));
      }
    }
    parts.push(this.encryptAndHash(payload));
    const out = concat(...parts);
    if (out.length > MAX_MESSAGE) throw new NoiseError();
    this.index += 1;
    return out;
  }

  readMessage(message: Uint8Array): Uint8Array {
    if (this.isFinished() || this.myTurn() || message.length > MAX_MESSAGE) throw new NoiseError();
    let pos = 0;
    const take = (n: number): Uint8Array => {
      if (pos + n > message.length) throw new NoiseError();
      const out = message.subarray(pos, pos + n);
      pos += n;
      return out;
    };
    for (const token of this.pattern.messages[this.index]) {
      switch (token) {
        case 'e': {
          if (this.re !== undefined) throw new NoiseError();
          this.re = take(DHLEN).slice();
          this.mixHash(this.re);
          if (this.pattern.hasPsk) this.mixKey(this.re);
          break;
        }
        case 's': {
          if (this.rs !== undefined) throw new NoiseError();
          const len = DHLEN + (this.cipher.hasKey() ? TAGLEN : 0);
          this.rs = this.decryptAndHash(take(len)).slice();
          break;
        }
        case 'psk':
          this.mixKeyAndHash(this.psk!);
          break;
        default:
          this.mixKey(this.dhToken(token));
      }
    }
    const payload = this.decryptAndHash(message.subarray(pos));
    this.index += 1;
    return payload;
  }

  /** The peer's static public key, once received (XX) or pre-known (KK). */
  remoteStatic(): Uint8Array | undefined {
    return this.rs;
  }

  /** Handshake hash, valid once finished. Used for the pairing code. */
  handshakeHash(): Uint8Array {
    if (!this.isFinished()) throw new NoiseError();
    return this.h;
  }

  /** Finish: derive the two transport cipher states. */
  split(): { send: CipherState; recv: CipherState } {
    if (!this.isFinished()) throw new NoiseError();
    const [k1, k2] = hkdf(this.ck, EMPTY, 2);
    const c1 = new CipherState();
    const c2 = new CipherState();
    c1.initializeKey(k1);
    c2.initializeKey(k2);
    return this.isInitiator ? { send: c1, recv: c2 } : { send: c2, recv: c1 };
  }
}

export { equalBytes };
