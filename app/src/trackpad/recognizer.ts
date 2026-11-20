// Trackpad gesture recognizer (PLAN.md §7). Plain TypeScript: no React Native imports, so it
// is unit-tested with simulated touch sequences. The UI feeds it the full list of touches on
// every touch event; it emits raw actions. Pointer speed, acceleration and the natural-scroll
// sign are applied by the caller.
//
//   one finger drag            -> move
//   tap                        -> left click
//   two-finger tap             -> right click
//   three-finger tap           -> three_tap (Play/Pause shortcut)
//   long press 350 ms, drag    -> left button held until lift (drag lock)
//   two-finger drag            -> scroll

export interface Touch {
  id: number;
  x: number;
  y: number;
}

export type TrackAction =
  | { type: 'move'; dx: number; dy: number }
  | { type: 'click'; button: 'left' | 'right'; count: 1 }
  | { type: 'button'; button: 'left'; state: 'down' | 'up' }
  | { type: 'scroll'; dx: number; dy: number }
  | { type: 'three_tap' };

export interface RecognizerOptions {
  /** Movement (px) before a touch stops being a tap. */
  slop: number;
  /** A tap must end within this many ms. */
  tapMaxMs: number;
  /** Hold this long without moving to start a drag lock. */
  longPressMs: number;
  /** `schedule(ms, fn)` returns a cancel function. Injected so tests can control time. */
  schedule: (ms: number, fn: () => void) => () => void;
}

export const DEFAULT_OPTIONS: Omit<RecognizerOptions, 'schedule'> = {
  slop: 10,
  tapMaxMs: 250,
  longPressMs: 350,
};

type Pos = { x: number; y: number };

export class Recognizer {
  private readonly o: RecognizerOptions;
  private last = new Map<number, Pos>();
  private start = new Map<number, Pos>();
  private active = false;
  private startTime = 0;
  private maxCount = 0;
  private moved = false;
  private locked = false;
  private cancelLongPress: (() => void) | null = null;

  constructor(
    private readonly emit: (a: TrackAction) => void,
    options: Partial<RecognizerOptions> & Pick<RecognizerOptions, 'schedule'>,
  ) {
    this.o = { ...DEFAULT_OPTIONS, ...options };
  }

  /** Call on every touch start, move and end with all touches currently on the screen. */
  update(touches: Touch[], now: number): void {
    if (touches.length === 0) {
      if (this.active) this.finish(now);
      return;
    }
    if (!this.active) this.begin(touches, now);

    const prev = this.last;
    const next = new Map<number, Pos>(touches.map((t) => [t.id, { x: t.x, y: t.y }]));
    for (const t of touches) if (!this.start.has(t.id)) this.start.set(t.id, { x: t.x, y: t.y });
    if (touches.length > this.maxCount) {
      this.maxCount = touches.length;
      if (this.maxCount >= 2) this.stopLongPress();
    }

    // Per-finger movement since the last update, for fingers present in both.
    const deltas: { dx: number; dy: number }[] = [];
    for (const t of touches) {
      const p = prev.get(t.id);
      if (p) deltas.push({ dx: t.x - p.x, dy: t.y - p.y });
    }
    const wasMoved = this.moved;
    if (!this.moved) {
      for (const t of touches) {
        const s = this.start.get(t.id)!;
        if (Math.hypot(t.x - s.x, t.y - s.y) > this.o.slop) this.moved = true;
      }
      if (this.moved) this.stopLongPress();
    }

    if (this.maxCount === 1 && touches.length === 1) {
      const t = touches[0];
      if (this.locked || this.moved) {
        const s = this.start.get(t.id)!;
        const p = prev.get(t.id);
        // On the first step past the slop, include the distance already travelled.
        const first = !this.locked && !wasMoved;
        const dx = first || !p ? t.x - s.x : t.x - p.x;
        const dy = first || !p ? t.y - s.y : t.y - p.y;
        if (dx !== 0 || dy !== 0) this.emit({ type: 'move', dx, dy });
      }
    } else if (this.maxCount === 2 && touches.length === 2 && this.moved && deltas.length === 2) {
      const dx = (deltas[0].dx + deltas[1].dx) / 2;
      const dy = (deltas[0].dy + deltas[1].dy) / 2;
      if (dx !== 0 || dy !== 0) this.emit({ type: 'scroll', dx, dy });
    }
    this.last = next;
  }

  /** The system cancelled the touches (e.g. a notification): drop everything, release a held button. */
  cancel(): void {
    if (this.locked) this.emit({ type: 'button', button: 'left', state: 'up' });
    this.reset();
  }

  private begin(touches: Touch[], now: number): void {
    this.active = true;
    this.startTime = now;
    this.maxCount = touches.length;
    this.moved = false;
    this.locked = false;
    this.last = new Map();
    this.start = new Map();
    if (touches.length === 1) {
      this.cancelLongPress = this.o.schedule(this.o.longPressMs, () => {
        this.cancelLongPress = null;
        if (this.active && !this.moved && this.maxCount === 1) {
          this.locked = true;
          this.emit({ type: 'button', button: 'left', state: 'down' });
        }
      });
    }
  }

  private finish(now: number): void {
    const duration = now - this.startTime;
    if (this.locked) {
      this.emit({ type: 'button', button: 'left', state: 'up' });
    } else if (!this.moved && duration <= this.o.tapMaxMs) {
      if (this.maxCount === 1) this.emit({ type: 'click', button: 'left', count: 1 });
      else if (this.maxCount === 2) this.emit({ type: 'click', button: 'right', count: 1 });
      else if (this.maxCount === 3) this.emit({ type: 'three_tap' });
    }
    this.reset();
  }

  private stopLongPress(): void {
    this.cancelLongPress?.();
    this.cancelLongPress = null;
  }

  private reset(): void {
    this.stopLongPress();
    this.active = false;
    this.locked = false;
    this.moved = false;
    this.maxCount = 0;
    this.last = new Map();
    this.start = new Map();
  }
}
