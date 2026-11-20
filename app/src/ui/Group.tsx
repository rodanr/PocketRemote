import { StyleSheet, Text, View } from 'react-native';
import { color, space } from '../theme';

/** A tiny muted title above a group of keys, so groups cannot be mistaken for one another. */
export function Group({ title, children, flex }: { title: string; children: React.ReactNode; flex?: number }) {
  return (
    <View style={{ flex }}>
      <Text style={s.title} accessibilityRole="header">
        {title}
      </Text>
      {children}
    </View>
  );
}

const s = StyleSheet.create({
  title: { fontSize: 11, fontWeight: '500', letterSpacing: 1, color: color.textMuted, marginBottom: space.xs, marginLeft: space.xs },
});
