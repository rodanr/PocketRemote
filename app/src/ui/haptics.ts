import * as Haptics from 'expo-haptics';

// The haptic grammar from docs/design-brief.md: tick for keys, click for taps,
// a firm one for drag lock, a heavy one when the connection is lost.
export const tick = () => void Haptics.selectionAsync();
export const click = () => void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
export const press = () => void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);
export const lock = () => void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Heavy);
export const lost = () => void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Warning);
