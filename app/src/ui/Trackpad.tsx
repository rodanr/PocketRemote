import { useEffect, useMemo, useState } from 'react';
import { AccessibilityInfo, Animated, type GestureResponderEvent, type LayoutChangeEvent, StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import type { PointerSettings } from '../trackpad/pointer';
import { color } from '../theme';
import { TrackpadController } from './trackpadController';

const MARGIN = 24; // dp kept clear of both side edges so Android's Back gesture cannot fire mid-drag
const PITCH = 24;

interface Props {
  send: (m: Message) => void;
  settings: PointerSettings;
  /** Lights drop to 10% when the remote is not usable (paused or disconnected). */
  dim?: boolean;
}

/**
 * The runway: two columns of aisle lights, small and faint at the top and larger and brighter
 * toward the thumb (perspective). Lights at the finger's height go to full; a drag lock turns
 * the whole runway solid. Nothing animates on its own.
 */
export function Trackpad({ send, settings, dim }: Props) {
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

  const rows = height > 0 ? Math.max(3, Math.floor((height - 32) / PITCH)) : 0;
  const dots = useMemo(
    () =>
      Array.from({ length: rows }, (_, i) => {
        const t = rows > 1 ? i / (rows - 1) : 1; // 0 at the top, 1 at the bottom
        const size = 3 + 2.5 * t;
        return { y: 16 + t * (height - 32) - size / 2, size, base: (dim ? 0.1 : 0.18) + (dim ? 0.1 : 0.4) * t };
      }),
    [rows, height, dim],
  );

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
        dots.map((d, i) => (
          <Animated.View
            key={`${side}${i}`}
            pointerEvents="none"
            style={[
              s.dot,
              side === 'left' ? { left: MARGIN } : { right: MARGIN },
              {
                top: d.y,
                width: d.size,
                height: d.size,
                borderRadius: d.size / 2,
                opacity: Animated.add(
                  ctl.fingerY.interpolate({ inputRange: [d.y - 100, d.y, d.y + 100], outputRange: [d.base, 1, d.base], extrapolate: 'clamp' }),
                  ctl.solid,
                ),
              },
            ]}
          />
        )),
      )}
    </View>
  );
}

const s = StyleSheet.create({
  pad: { flex: 1, minHeight: 160, borderRadius: 24, backgroundColor: color.trackpad, borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.hairline, overflow: 'hidden' },
  dot: { position: 'absolute', backgroundColor: color.amber },
});
