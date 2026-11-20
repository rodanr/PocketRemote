// Post-handshake transport: one Noise transport message per WebSocket binary frame.
import { CipherState, NoiseError, TAGLEN } from './primitives';

/** Post-handshake frames are at most this many bytes (PLAN.md §3.9). */
export const MAX_TRANSPORT_FRAME = 4096;
/** Rekey a direction after this many messages (PLAN.md §4.1). Must equal the Rust side. */
export const REKEY_INTERVAL = 1 << 20;

const EMPTY = new Uint8Array(0);

export class Transport {
  private sent = 0;
  private received = 0;

  constructor(
    private readonly send: CipherState,
    private readonly recv: CipherState,
    private readonly rekeyInterval: number = REKEY_INTERVAL,
  ) {}

  encrypt(plaintext: Uint8Array): Uint8Array {
    if (plaintext.length + TAGLEN > MAX_TRANSPORT_FRAME) throw new NoiseError();
    const out = this.send.encryptWithAd(EMPTY, plaintext);
    this.sent += 1;
    if (this.sent === this.rekeyInterval) {
      this.send.rekey();
      this.sent = 0;
    }
    return out;
  }

  /** Any failure (bad tag, replay, reorder, oversize) must close the session. */
  decrypt(frame: Uint8Array): Uint8Array {
    if (frame.length > MAX_TRANSPORT_FRAME || frame.length < TAGLEN) throw new NoiseError();
    const out = this.recv.decryptWithAd(EMPTY, frame);
    this.received += 1;
    if (this.received === this.rekeyInterval) {
      this.recv.rekey();
      this.received = 0;
    }
    return out;
  }
}
