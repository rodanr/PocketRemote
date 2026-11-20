import { StyleSheet, View } from 'react-native';
import { Text } from './Text';
import { color, space } from '../theme';

/** A tiny muted title above a group of keys, so groups cannot be mistaken for one another. */
export function Group({ title, children, flex }: { title: string; children: React.ReactNode; flex?: number }) {
  return (
    <View style={{ flex }}>
      <Text style={s.title} accessibilityRole="header">
        {title}
      </Text>
      <View style={flex ? { flex: 1 } : undefined}>{children}</View>
    </View>
  );
}

const s = StyleSheet.create({
  title: { fontSize: 12, fontWeight: '500', letterSpacing: 0.4, color: color.textMuted, marginBottom: space.xs, marginLeft: space.xs },
});
