// Imperative half of the trackpad: gesture recognition, pointer gain, per-frame batching,
// haptics and the light animation values. Kept out of the component so rendering stays pure.
import { Animated } from 'react-native';
import type { Message } from '../protocol';
import { gain, MoveBatcher, scrollAmount, type AppSettings } from '../trackpad/pointer';
import { Recognizer, type TrackAction, type Touch } from '../trackpad/recognizer';
import { click, lock, press } from './haptics';

export class TrackpadController {
  send: (m: Message) => void;
  settings: AppSettings;
  reduceMotion = false;
  /** Y of the first finger inside the pad; far outside when no finger is down. */
  readonly fingerY = new Animated.Value(-1000);
  /** 1 while a drag lock holds the left button: the whole runway goes solid. */
  readonly solid = new Animated.Value(0);

  private lastMove = 0;
  private readonly rec: Recognizer;

  constructor(send: (m: Message) => void, settings: AppSettings) {
    this.send = send;
    this.settings = settings;
    const moves = new MoveBatcher(
      (dx, dy) => this.send({ type: 'move', dx, dy }),
      (cb) => requestAnimationFrame(cb),
    );
    const scrolls = new MoveBatcher(
      (dx, dy) => this.send({ type: 'scroll', dx, dy }),
      (cb) => requestAnimationFrame(cb),
    );
    this.rec = new Recognizer(
      (a) => this.onAction(a, moves, scrolls),
      {
        schedule: (ms, fn) => {
          const h = setTimeout(fn, ms);
          return () => clearTimeout(h);
        },
      },
    );
  }

  private onAction(a: TrackAction, moves: MoveBatcher, scrolls: MoveBatcher): void {
    switch (a.type) {
      case 'move': {
        const now = Date.now();
        const g = gain(Math.hypot(a.dx, a.dy), now - this.lastMove, this.settings);
        this.lastMove = now;
        moves.add(a.dx * g, a.dy * g);
        break;
      }
      case 'scroll': {
        const v = scrollAmount(a.dx, a.dy, this.settings);
        scrolls.add(v.dx, v.dy);
        break;
      }
      case 'click':
        click();
        this.send({ type: 'click', button: a.button, count: 1 });
        break;
      case 'button':
        lock();
        this.send({ type: 'button', button: a.button, state: a.state });
        Animated.timing(this.solid, {
          toValue: a.state === 'down' ? 1 : 0,
          duration: this.reduceMotion ? 0 : 120,
          useNativeDriver: true,
        }).start();
        break;
      case 'three_tap':
        press();
        this.send({ type: 'media', action: 'play_pause' });
        break;
    }
  }

  configure(send: (m: Message) => void, settings: AppSettings): void {
    this.send = send;
    this.settings = settings;
  }

  setReduceMotion(v: boolean): void {
    this.reduceMotion = v;
  }

  /** `touches` are all fingers now on the pad; `firstY` is the first finger's y inside the pad. */
  feed(touches: Touch[], firstY: number | null): void {
    this.fingerY.setValue(touches.length && firstY !== null ? firstY : -1000);
    if (!touches.length) this.lastMove = 0;
    this.rec.update(touches, Date.now());
  }

  cancel(): void {
    this.rec.cancel();
    this.fingerY.setValue(-1000);
  }
}
