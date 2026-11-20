import MaterialCommunityIcons from '@expo/vector-icons/MaterialCommunityIcons';
import { useEffect, useRef, useState } from 'react';
import { Animated, Pressable, StyleSheet, Text, type StyleProp, type ViewStyle } from 'react-native';
import { color, radius, TARGET } from '../theme';
import { tick } from './haptics';

type IconName = React.ComponentProps<typeof MaterialCommunityIcons>['name'];

export interface KeyProps {
  /** Spoken by TalkBack: say the action in words ("Previous tab"). */
  label: string;
  icon?: IconName;
  iconSize?: number;
  /** Visible text. Keys on the same page use the same text size. */
  text?: string;
  textSize?: number;
  onPress: () => void;
  /** The one lead action on the screen: amber. */
  lead?: boolean;
  /** Repeat while held (volume). */
  repeat?: boolean;
  /** Filled but red-tinted, so a key that cannot be undone is easy to tell apart (Close tab). */
  danger?: boolean;
  flex?: number;
  height?: number;
  style?: StyleProp<ViewStyle>;
}

const REPEAT_DELAY = 350;
const REPEAT_EVERY = 110;

/**
 * One key style everywhere: a filled 16 dp rounded rectangle with a faint rim, and a soft spring
 * when pressed. Only the lead key is amber; a danger key gets a red tint.
 */
export function Key({ label, icon, iconSize = 24, text, textSize = 15, onPress, lead, repeat, danger, flex = 1, height = 48, style }: KeyProps) {
  const timer = useRef<ReturnType<typeof setTimeout> | ReturnType<typeof setInterval> | null>(null);
  const [pressed, setPressed] = useState(false);
  const [scale] = useState(() => new Animated.Value(1));

  const stop = () => {
    if (timer.current !== null) {
      clearTimeout(timer.current as ReturnType<typeof setTimeout>);
      clearInterval(timer.current as ReturnType<typeof setInterval>);
      timer.current = null;
    }
  };
  useEffect(() => stop, []);

  const fire = () => {
    tick();
    onPress();
  };
  const spring = (to: number, speed: number, bounciness: number) =>
    Animated.spring(scale, { toValue: to, speed, bounciness, useNativeDriver: true }).start();

  const face = danger ? (pressed ? s.dangerPressed : s.danger) : lead ? (pressed ? s.leadPressed : s.lead) : pressed ? s.pressed : s.normal;
  const ink = lead ? color.onAmber : danger ? color.danger : color.text;

  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={repeat ? undefined : fire}
      onPressIn={() => {
        setPressed(true);
        spring(0.96, 50, 0);
        if (repeat) {
          fire();
          timer.current = setTimeout(() => {
            timer.current = setInterval(fire, REPEAT_EVERY);
          }, REPEAT_DELAY);
        }
      }}
      onPressOut={() => {
        setPressed(false);
        spring(1, 24, 6);
        if (repeat) stop();
      }}
      style={[{ flex, minHeight: Math.max(height, TARGET) }, style]}
    >
      <Animated.View style={[s.face, face, { transform: [{ scale }] }]}>
        {icon ? <MaterialCommunityIcons name={icon} size={iconSize} color={ink} /> : null}
        {text ? (
          <Text style={[s.text, { fontSize: textSize, color: ink }]} numberOfLines={2}>
            {text}
          </Text>
        ) : null}
      </Animated.View>
    </Pressable>
  );
}

const s = StyleSheet.create({
  face: { flex: 1, borderRadius: radius.m, borderWidth: 1, alignItems: 'center', justifyContent: 'center', flexDirection: 'row', gap: 6, paddingHorizontal: 6 },
  normal: { backgroundColor: color.surface, borderColor: color.rim },
  pressed: { backgroundColor: color.surfacePressed, borderColor: color.rim },
  lead: { backgroundColor: color.amber, borderColor: '#c98a2e' },
  leadPressed: { backgroundColor: '#e89f3e', borderColor: '#c98a2e' },
  danger: { backgroundColor: color.surface, borderColor: 'rgba(255,122,106,0.6)' },
  dangerPressed: { backgroundColor: 'rgba(255,122,106,0.16)', borderColor: 'rgba(255,122,106,0.6)' },
  text: { fontWeight: '500', letterSpacing: 0.1, flexShrink: 1, textAlign: 'center' },
});
