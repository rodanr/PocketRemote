import { MoveBatcher, DEFAULT_SETTINGS, gain, scrollAmount } from '../pointer';
import { Recognizer, type TrackAction, type Touch } from '../recognizer';

/** Fake clock and scheduler so long-press timing is deterministic. */
function rig() {
  let now = 0;
  const timers: { at: number; fn: () => void; live: boolean }[] = [];
  const actions: TrackAction[] = [];
  const r = new Recognizer((a) => actions.push(a), {
    schedule: (ms, fn) => {
      const t = { at: now + ms, fn, live: true };
      timers.push(t);
      return () => {
        t.live = false;
      };
    },
  });
  const advance = (ms: number) => {
    now += ms;
    for (const t of timers) if (t.live && t.at <= now) { t.live = false; t.fn(); }
  };
  const touch = (...pts: Touch[]) => r.update(pts, now);
  const t = (id: number, x: number, y: number): Touch => ({ id, x, y });
  return { r, actions, advance, touch, t, now: () => now };
}

describe('Recognizer', () => {
  test('one-finger drag emits moves, including the distance before the slop', () => {
    const g = rig();
    g.touch(g.t(1, 100, 100));
    g.advance(16);
    g.touch(g.t(1, 120, 100)); // 20 px: past the 10 px slop
    g.advance(16);
    g.touch(g.t(1, 125, 103));
    g.touch();
    expect(g.actions).toEqual([
      { type: 'move', dx: 20, dy: 0 },
      { type: 'move', dx: 5, dy: 3 },
    ]);
  });

  test('a short stationary touch is a left click; a long one is not', () => {
    let g = rig();
    g.touch(g.t(1, 50, 50));
    g.advance(120);
    g.touch();
    expect(g.actions).toEqual([{ type: 'click', button: 'left', count: 1 }]);
    g = rig();
    g.touch(g.t(1, 50, 50));
    g.advance(300); // past the 250 ms tap limit but before the 350 ms long press
    g.touch();
    expect(g.actions).toEqual([]);
  });

  test('tiny jitter below the slop still counts as a tap', () => {
    const g = rig();
    g.touch(g.t(1, 50, 50));
    g.advance(20);
    g.touch(g.t(1, 54, 52));
    g.advance(60);
    g.touch();
    expect(g.actions).toEqual([{ type: 'click', button: 'left', count: 1 }]);
  });

  test('long press then drag holds the button until the finger lifts', () => {
    const g = rig();
    g.touch(g.t(1, 10, 10));
    g.advance(349);
    expect(g.actions).toEqual([]);
    g.advance(2); // 351 ms
    expect(g.actions).toEqual([{ type: 'button', button: 'left', state: 'down' }]);
    g.touch(g.t(1, 14, 10)); // moves count immediately while locked, with no slop
    g.touch(g.t(1, 20, 18));
    g.touch();
    expect(g.actions).toEqual([
      { type: 'button', button: 'left', state: 'down' },
      { type: 'move', dx: 4, dy: 0 },
      { type: 'move', dx: 6, dy: 8 },
      { type: 'button', button: 'left', state: 'up' },
    ]);
  });

  test('moving before 350 ms cancels the long press', () => {
    const g = rig();
    g.touch(g.t(1, 10, 10));
    g.advance(100);
    g.touch(g.t(1, 40, 10));
    g.advance(400);
    g.touch();
    expect(g.actions.some((a) => a.type === 'button')).toBe(false);
  });

  test('two-finger tap is a right click', () => {
    const g = rig();
    g.touch(g.t(1, 50, 50));
    g.advance(10);
    g.touch(g.t(1, 50, 50), g.t(2, 90, 50));
    g.advance(80);
    g.touch(g.t(2, 90, 50)); // one lifts first
    g.advance(10);
    g.touch();
    expect(g.actions).toEqual([{ type: 'click', button: 'right', count: 1 }]);
  });

  test('two-finger drag scrolls with the mean of both fingers, and no pointer move', () => {
    const g = rig();
    g.touch(g.t(1, 50, 100), g.t(2, 90, 100));
    g.advance(16);
    g.touch(g.t(1, 50, 120), g.t(2, 90, 124)); // 20 px: past the slop
    g.advance(16);
    g.touch(g.t(1, 50, 130), g.t(2, 90, 134));
    g.touch();
    expect(g.actions).toEqual([
      { type: 'scroll', dx: 0, dy: 22 },
      { type: 'scroll', dx: 0, dy: 10 },
    ]);
  });

  test('a second finger arriving during a one-finger drag does not start a click or drag lock', () => {
    const g = rig();
    g.touch(g.t(1, 0, 0));
    g.touch(g.t(1, 30, 0));
    g.advance(20);
    g.touch(g.t(1, 30, 0), g.t(2, 80, 0));
    g.advance(500);
    g.touch();
    expect(g.actions.filter((a) => a.type !== 'move')).toEqual([]);
  });

  test('three-finger tap is the Play/Pause shortcut', () => {
    const g = rig();
    g.touch(g.t(1, 10, 50), g.t(2, 50, 50), g.t(3, 90, 50));
    g.advance(90);
    g.touch();
    expect(g.actions).toEqual([{ type: 'three_tap' }]);
  });

  test('cancel releases a held button and clears state', () => {
    const g = rig();
    g.touch(g.t(1, 10, 10));
    g.advance(400);
    g.r.cancel();
    expect(g.actions).toEqual([
      { type: 'button', button: 'left', state: 'down' },
      { type: 'button', button: 'left', state: 'up' },
    ]);
    g.touch(g.t(1, 10, 10));
    g.advance(50);
    g.touch();
    expect(g.actions.at(-1)).toEqual({ type: 'click', button: 'left', count: 1 });
  });
});

describe('pointer helpers', () => {
  test('acceleration: faster swipes get a bigger gain, capped', () => {
    const slow = gain(2, 16, DEFAULT_SETTINGS);
    const fast = gain(60, 16, DEFAULT_SETTINGS);
    const crazy = gain(600, 16, DEFAULT_SETTINGS);
    expect(fast).toBeGreaterThan(slow);
    expect(crazy).toBeCloseTo(DEFAULT_SETTINGS.speed * (1 + DEFAULT_SETTINGS.acceleration * 2));
    expect(gain(5, 16, { ...DEFAULT_SETTINGS, acceleration: 0 })).toBe(DEFAULT_SETTINGS.speed);
  });

  test('natural scroll flips the sign', () => {
    expect(scrollAmount(3, 10, { ...DEFAULT_SETTINGS, naturalScroll: true })).toEqual({ dx: -3, dy: -10 });
    expect(scrollAmount(3, 10, { ...DEFAULT_SETTINGS, naturalScroll: false })).toEqual({ dx: 3, dy: 10 });
  });

  test('batcher sends one move per frame and nothing when idle', () => {
    const sent: [number, number][] = [];
    const frames: (() => void)[] = [];
    const b = new MoveBatcher((dx, dy) => sent.push([dx, dy]), (cb) => frames.push(cb));
    b.add(1, 2);
    b.add(3, 4);
    expect(frames.length).toBe(1); // one frame scheduled, not two
    frames.shift()!();
    expect(sent).toEqual([[4, 6]]);
    b.add(2, 3);
    b.add(-2, -3); // cancels out within one frame: nothing is sent
    frames.shift()!();
    expect(sent).toEqual([[4, 6]]);
    expect(frames.length).toBe(0);
  });
});
