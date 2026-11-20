import { MaterialCommunityIcons } from '@expo/vector-icons';
import { useEffect, useMemo, useState } from 'react';
import { AccessibilityInfo, Animated, type GestureResponderEvent, type LayoutChangeEvent, Pressable, StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import type { PointerSettings } from '../trackpad/pointer';
import { color, radius, space } from '../theme';
import { press } from './haptics';
import { TrackpadController } from './trackpadController';

const MARGIN = 24; // dp kept clear of both side edges so Android's Back gesture cannot fire mid-drag
const SPACING = 26;
const BASE = 0.35; // idle light level; the lights never animate on their own

interface Props {
  send: (m: Message) => void;
  settings: PointerSettings;
}

export function Trackpad({ send, settings }: Props) {
  const [ctl] = useState(() => new TrackpadController(send, settings));
  const [height, setHeight] = useState(0);

  useEffect(() => {
    ctl.configure(send, settings);
  }, [ctl, send, settings]);

  useEffect(() => {
    void AccessibilityInfo.isReduceMotionEnabled().then((v) => ctl.setReduceMotion(v));
  }, [ctl]);

  const feed = (e: GestureResponderEvent) => {
    const touches = e.nativeEvent.touches.map((t) => ({ id: Number(t.identifier), x: t.pageX, y: t.pageY }));
    ctl.feed(touches, touches.length ? e.nativeEvent.touches[0].locationY : null);
  };

  const rows = height > 0 ? Math.max(2, Math.floor((height - 24) / SPACING)) : 0;
  const dots = useMemo(() => Array.from({ length: rows }, (_, i) => 12 + i * ((height - 24) / Math.max(1, rows - 1))), [rows, height]);

  return (
    <View
      style={s.pad}
      onLayout={(e: LayoutChangeEvent) => setHeight(e.nativeEvent.layout.height)}
      onStartShouldSetResponder={() => true}
      onMoveShouldSetResponder={() => true}
      onResponderTerminationRequest={() => false}
      onTouchStart={feed}
      onTouchMove={feed}
      onTouchEnd={feed}
      onTouchCancel={() => ctl.cancel()}
      accessible
      accessibilityLabel="Trackpad"
      accessibilityHint="Drag to move the pointer. Double tap to click."
      accessibilityActions={[
        { name: 'activate', label: 'Left click' },
        { name: 'longpress', label: 'Right click' },
      ]}
      onAccessibilityAction={(e) => {
        if (e.nativeEvent.actionName === 'activate') send({ type: 'click', button: 'left', count: 1 });
        else if (e.nativeEvent.actionName === 'longpress') send({ type: 'click', button: 'right', count: 1 });
      }}
    >
      {(['left', 'right'] as const).map((side) =>
        dots.map((y, i) => (
          <Animated.View
            key={`${side}${i}`}
            pointerEvents="none"
            style={[
              s.dot,
              side === 'left' ? { left: MARGIN - 3 } : { right: MARGIN - 3 },
              {
                top: y,
                opacity: Animated.add(
                  ctl.fingerY.interpolate({ inputRange: [y - 90, y, y + 90], outputRange: [BASE, 1, BASE], extrapolate: 'clamp' }),
                  ctl.solid,
                ),
              },
            ]}
          />
        )),
      )}
      {/* Close tab: its own corner, outlined, needs a deliberate hold (it cannot be undone). */}
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Close tab"
        accessibilityHint="Press and hold"
        delayLongPress={300}
        onLongPress={() => {
          press();
          send({ type: 'browser', action: 'close_tab' });
        }}
        style={({ pressed }) => [s.close, pressed && { backgroundColor: color.surfacePressed }]}
      >
        <MaterialCommunityIcons name="tab-remove" size={22} color={color.textMuted} />
      </Pressable>
    </View>
  );
}

const s = StyleSheet.create({
  pad: { flex: 1, minHeight: 160, borderRadius: radius.pill, backgroundColor: color.trackpad, borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline, overflow: 'hidden' },
  dot: { position: 'absolute', width: 6, height: 6, borderRadius: 3, backgroundColor: color.amber },
  close: { position: 'absolute', top: space.s, right: MARGIN + 12, width: 48, height: 48, borderRadius: 24, alignItems: 'center', justifyContent: 'center', borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline },
});
