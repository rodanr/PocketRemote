import * as Haptics from 'expo-haptics';

// The haptic grammar from docs/design-brief.md: tick for keys, click for taps,
// a firm one for drag lock, a heavy one when the connection is lost.
// The Settings page can switch vibration off: every call below then does nothing.
let enabled = true;
export const setHapticsEnabled = (on: boolean) => {
  enabled = on;
};

export const tick = () => {
  if (enabled) void Haptics.selectionAsync();
};
export const click = () => {
  if (enabled) void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
};
export const press = () => {
  if (enabled) void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);
};
export const lock = () => {
  if (enabled) void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Heavy);
};
export const lost = () => {
  if (enabled) void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Warning);
};
