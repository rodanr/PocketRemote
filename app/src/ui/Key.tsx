import MaterialCommunityIcons from '@expo/vector-icons/MaterialCommunityIcons';
import { useEffect, useRef } from 'react';
import { Pressable, StyleSheet, Text, type StyleProp, type ViewStyle } from 'react-native';
import { color, radius, TARGET, type as t } from '../theme';
import { tick } from './haptics';

type IconName = React.ComponentProps<typeof MaterialCommunityIcons>['name'];

export interface KeyProps {
  /** Spoken by TalkBack: say the action in words ("Previous tab"). */
  label: string;
  icon?: IconName;
  /** Put the icon after the text (Forward →, Tab ›). */
  iconAfter?: boolean;
  /** Short visible text, used with or instead of an icon. */
  text?: string;
  onPress: () => void;
  /** The one lead action on the screen: amber. */
  lead?: boolean;
  /** Repeat while held (volume). */
  repeat?: boolean;
  /** Needs a deliberate hold (Close tab): fires after this many ms. */
  holdMs?: number;
  flex?: number;
  height?: number;
  round?: boolean;
  /** Unfilled, outlined in the danger colour (Close tab). */
  danger?: boolean;
  style?: StyleProp<ViewStyle>;
}

const REPEAT_DELAY = 350;
const REPEAT_EVERY = 110;

export function Key({ label, icon, iconAfter, text, onPress, lead, repeat, holdMs, flex = 1, height = 52, round, danger, style }: KeyProps) {
  const timer = useRef<ReturnType<typeof setTimeout> | ReturnType<typeof setInterval> | null>(null);
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

  const glyph = icon ? <MaterialCommunityIcons name={icon} size={24} color={lead ? color.onAmber : danger ? color.danger : color.text} /> : null;

  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityHint={holdMs ? 'Press and hold' : undefined}
      onPress={holdMs || repeat ? undefined : fire}
      onLongPress={holdMs ? fire : undefined}
      delayLongPress={holdMs}
      onPressIn={
        repeat
          ? () => {
              fire();
              timer.current = setTimeout(() => {
                timer.current = setInterval(fire, REPEAT_EVERY);
              }, REPEAT_DELAY);
            }
          : undefined
      }
      onPressOut={repeat ? stop : undefined}
      style={({ pressed }) => [
        s.base,
        { flex, minHeight: Math.max(height, TARGET), borderRadius: round ? 999 : radius.m },
        danger ? s.danger : lead ? s.lead : s.normal,
        pressed && (lead ? s.leadPressed : s.pressed),
        style,
      ]}
    >
      {icon && !iconAfter ? glyph : null}
      {text ? (
        <Text
          style={[s.text, lead && { color: color.onAmber }, danger && { color: color.danger }]}
          numberOfLines={1}
          adjustsFontSizeToFit
          minimumFontScale={0.75}
        >
          {text}
        </Text>
      ) : null}
      {icon && iconAfter ? glyph : null}
    </Pressable>
  );
}

const s = StyleSheet.create({
  base: { alignItems: 'center', justifyContent: 'center', flexDirection: 'row', gap: 6, paddingHorizontal: 6 },
  // Filled keys carry no outline: fill plus outline says 'shape' twice and looks fussy.
  normal: { backgroundColor: color.surface },
  danger: { borderWidth: StyleSheet.hairlineWidth * 2, borderColor: 'rgba(255,107,90,0.6)', backgroundColor: 'transparent' },
  pressed: { backgroundColor: color.surfacePressed },
  lead: { backgroundColor: color.amber },
  leadPressed: { backgroundColor: '#e89f3e' },
  text: { ...t.body, fontSize: 14, flexShrink: 1 },
});
