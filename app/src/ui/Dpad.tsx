import MaterialCommunityIcons from '@expo/vector-icons/MaterialCommunityIcons';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import type { KeyName, Message } from '../protocol';
import { color, space } from '../theme';
import { tick } from './haptics';

const SIZE = 160; // dp circle (brief: 160 dp with a 64 dp OK)
const OK = 64;

function Arrow({ name, label, k, send, style }: { name: 'chevron-up' | 'chevron-down' | 'chevron-left' | 'chevron-right'; label: string; k: KeyName; send: (m: Message) => void; style: object }) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={() => {
        tick();
        send({ type: 'key', key: k });
      }}
      style={({ pressed }) => [s.arrow, style, pressed && { backgroundColor: color.surfacePressed }]}
    >
      <MaterialCommunityIcons name={name} size={26} color={color.text} />
    </Pressable>
  );
}

/** Round D-pad: four arrows around a 64 dp OK (Enter). Glyphs are warm text, not amber. */
export function Dpad({ send }: { send: (m: Message) => void }) {
  const edge = (SIZE - 48) / 2;
  return (
    <View style={s.pad} accessibilityLabel="Direction pad">
      <Arrow name="chevron-up" label="Up" k="up" send={send} style={{ top: 2, left: edge }} />
      <Arrow name="chevron-down" label="Down" k="down" send={send} style={{ bottom: 2, left: edge }} />
      <Arrow name="chevron-left" label="Left" k="left" send={send} style={{ left: 2, top: edge }} />
      <Arrow name="chevron-right" label="Right" k="right" send={send} style={{ right: 2, top: edge }} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="OK, press Enter"
        onPress={() => {
          tick();
          send({ type: 'key', key: 'enter' });
        }}
        style={({ pressed }) => [s.ok, pressed && { backgroundColor: color.surfacePressed }]}
      >
        <Text style={s.okText}>OK</Text>
      </Pressable>
    </View>
  );
}

const s = StyleSheet.create({
  pad: { width: SIZE, height: SIZE, borderRadius: SIZE / 2, backgroundColor: color.surface },
  arrow: { position: 'absolute', width: 48, height: 48, borderRadius: 24, alignItems: 'center', justifyContent: 'center' },
  ok: { position: 'absolute', left: (SIZE - OK) / 2 - 1, top: (SIZE - OK) / 2 - 1, width: OK, height: OK, borderRadius: OK / 2, alignItems: 'center', justifyContent: 'center', backgroundColor: color.ground, borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline, margin: space.xs - 4 },
  okText: { color: color.text, fontSize: 16, fontWeight: '500' },
});
