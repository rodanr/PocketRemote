// Pointer acceleration and per-frame move batching. Plain TypeScript.

export interface AppSettings {
  /** Base multiplier. 1 = one phone pixel moves the cursor one pixel. */
  speed: number;
  /** 0 = linear, 1 = strong acceleration. */
  acceleration: number;
  naturalScroll: boolean;
  /** Scroll multiplier on top of the phone's finger distance. 1 = default. */
  scrollSpeed: number;
  /** Vibration feedback on keys, taps and connection loss. */
  haptics: boolean;
  /** Keep the phone screen on while the remote is open. */
  keepAwake: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  speed: 1.6,
  acceleration: 0.5,
  naturalScroll: true,
  scrollSpeed: 1,
  haptics: true,
  keepAwake: true,
};

/** Gain for a move of `distance` px that took `dtMs` ms. Faster swipes travel further. */
export function gain(distance: number, dtMs: number, s: AppSettings): number {
  const v = dtMs > 0 ? distance / dtMs : 0; // px per ms
  return s.speed * (1 + s.acceleration * Math.min(v / 2, 2));
}

/** Scroll amounts: natural scroll moves content with the fingers. */
export function scrollAmount(dx: number, dy: number, s: AppSettings): { dx: number; dy: number } {
  const k = s.naturalScroll ? -1 : 1;
  return { dx: k * dx * s.scrollSpeed, dy: k * dy * s.scrollSpeed };
}

/**
 * Collects pointer deltas and sends at most one `move` per animation frame (and none when idle).
 * `raf` schedules the next frame; it is `requestAnimationFrame` in the app.
 */
export class MoveBatcher {
  private dx = 0;
  private dy = 0;
  private pending = false;

  constructor(
    private readonly send: (dx: number, dy: number) => void,
    private readonly raf: (cb: () => void) => void,
  ) {}

  add(dx: number, dy: number): void {
    this.dx += dx;
    this.dy += dy;
    if (!this.pending) {
      this.pending = true;
      this.raf(() => this.flush());
    }
  }

  flush(): void {
    this.pending = false;
    const { dx, dy } = this;
    this.dx = 0;
    this.dy = 0;
    if (dx !== 0 || dy !== 0) this.send(dx, dy);
  }
}
