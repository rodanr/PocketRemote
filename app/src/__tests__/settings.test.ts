import * as Haptics from 'expo-haptics';
import { DEFAULT_SETTINGS, scrollAmount } from '../trackpad/pointer';
import * as h from '../ui/haptics';

jest.mock('expo-haptics', () => ({
  selectionAsync: jest.fn(),
  impactAsync: jest.fn(),
  notificationAsync: jest.fn(),
  ImpactFeedbackStyle: { Light: 'l', Medium: 'm', Heavy: 'h' },
  NotificationFeedbackType: { Warning: 'w' },
}));

describe('scroll speed', () => {
  test('scales scrolling on top of natural direction', () => {
    expect(scrollAmount(10, 20, { ...DEFAULT_SETTINGS, scrollSpeed: 1 })).toEqual({ dx: -10, dy: -20 });
    expect(scrollAmount(10, 20, { ...DEFAULT_SETTINGS, scrollSpeed: 2 })).toEqual({ dx: -20, dy: -40 });
    expect(scrollAmount(10, 20, { ...DEFAULT_SETTINGS, naturalScroll: false, scrollSpeed: 0.5 })).toEqual({ dx: 5, dy: 10 });
  });
});

describe('vibration switch', () => {
  test('does nothing when off and works again when on', () => {
    h.setHapticsEnabled(false);
    h.tick();
    h.click();
    h.press();
    h.lock();
    h.lost();
    expect(Haptics.selectionAsync).not.toHaveBeenCalled();
    expect(Haptics.impactAsync).not.toHaveBeenCalled();
    h.setHapticsEnabled(true);
    h.tick();
    expect(Haptics.selectionAsync).toHaveBeenCalledTimes(1);
  });
});
