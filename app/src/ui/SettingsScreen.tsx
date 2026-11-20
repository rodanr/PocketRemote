import MaterialCommunityIcons from '@expo/vector-icons/MaterialCommunityIcons';
import { Pressable, ScrollView, StyleSheet, Switch, Text, View } from 'react-native';
import type { PointerSettings } from '../trackpad/pointer';
import { color, space, TARGET, type as t } from '../theme';
import { tick } from './haptics';

interface Props {
  settings: PointerSettings;
  hostName: string;
  onChange: (s: PointerSettings) => void;
  onBack: () => void;
  onForget: () => void;
}

const GESTURES: [string, string][] = [
  ['One finger', 'Move the pointer'],
  ['Tap', 'Left click'],
  ['Two-finger tap', 'Right click'],
  ['Press and hold, then drag', 'Drag'],
  ['Two fingers', 'Scroll'],
  ['Three-finger tap', 'Play / Pause'],
];

function Stepper({ label, value, display, onDec, onInc }: { label: string; value: number; display: string; onDec: () => void; onInc: () => void }) {
  return (
    <View style={s.row}>
      <Text style={t.body}>{label}</Text>
      <View style={s.stepper} accessibilityRole="adjustable" accessibilityLabel={label} accessibilityValue={{ text: display }}>
        <Pressable accessibilityLabel={`Decrease ${label}`} onPress={() => { tick(); onDec(); }} style={s.step}>
          <Text style={s.stepText}>−</Text>
        </Pressable>
        <Text style={[t.body, { minWidth: 44, textAlign: 'center' }]} aria-hidden={value < 0}>{display}</Text>
        <Pressable accessibilityLabel={`Increase ${label}`} onPress={() => { tick(); onInc(); }} style={s.step}>
          <Text style={s.stepText}>+</Text>
        </Pressable>
      </View>
    </View>
  );
}

const round = (v: number) => Math.round(v * 100) / 100;

/** Full-screen settings page with a back arrow (the Android Back button also returns). */
export function SettingsScreen({ settings, hostName, onChange, onBack, onForget }: Props) {
  return (
    <View style={s.screen}>
      <View style={s.top}>
        <Pressable accessibilityRole="button" accessibilityLabel="Back" onPress={onBack} style={s.back}>
          <MaterialCommunityIcons name="arrow-left" size={24} color={color.text} />
        </Pressable>
        <Text style={t.title} accessibilityRole="header">Settings</Text>
      </View>
      <ScrollView contentContainerStyle={s.content}>
        <Stepper label="Pointer speed" value={settings.speed} display={`${settings.speed.toFixed(2)}×`} onDec={() => onChange({ ...settings, speed: Math.max(0.5, round(settings.speed - 0.25)) })} onInc={() => onChange({ ...settings, speed: Math.min(4, round(settings.speed + 0.25)) })} />
        <Stepper label="Acceleration" value={settings.acceleration} display={`${Math.round(settings.acceleration * 100)}%`} onDec={() => onChange({ ...settings, acceleration: Math.max(0, round(settings.acceleration - 0.25)) })} onInc={() => onChange({ ...settings, acceleration: Math.min(1, round(settings.acceleration + 0.25)) })} />
        <View style={s.row}>
          <Text style={t.body}>Natural scroll</Text>
          <Switch value={settings.naturalScroll} onValueChange={(v) => onChange({ ...settings, naturalScroll: v })} trackColor={{ true: color.amber, false: color.surface }} thumbColor={color.text} accessibilityLabel="Natural scroll" />
        </View>
        <Text style={[t.label, { marginTop: space.m, marginBottom: space.s }]} accessibilityRole="header">
          TRACKPAD GESTURES
        </Text>
        {GESTURES.map(([how, what]) => (
          <View key={how} style={s.gesture} accessible accessibilityLabel={`${how}: ${what}`}>
            <Text style={s.how}>{how}</Text>
            <Text style={s.what}>{what}</Text>
          </View>
        ))}
        <Pressable accessibilityRole="button" onPress={onForget} style={s.forget}>
          <Text style={{ color: color.danger, fontSize: 15 }}>Forget {hostName} and pair again</Text>
        </Pressable>
      </ScrollView>
    </View>
  );
}

const s = StyleSheet.create({
  screen: { flex: 1, backgroundColor: color.ground },
  top: { flexDirection: 'row', alignItems: 'center', gap: space.s, paddingHorizontal: space.s, paddingTop: space.s, minHeight: 56 },
  back: { width: TARGET, height: TARGET, alignItems: 'center', justifyContent: 'center' },
  content: { padding: space.l, paddingTop: space.m },
  row: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', minHeight: 56 },
  stepper: { flexDirection: 'row', alignItems: 'center' },
  step: { width: TARGET, height: TARGET, borderRadius: TARGET / 2, alignItems: 'center', justifyContent: 'center', backgroundColor: color.surface, borderWidth: 1, borderColor: color.rim },
  stepText: { color: color.text, fontSize: 22 },
  gesture: { flexDirection: 'row', justifyContent: 'space-between', paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: '#1f1b15' },
  how: { ...t.body, fontSize: 14, color: color.text },
  what: { ...t.body, fontSize: 14, color: color.textMuted },
  forget: { minHeight: TARGET, justifyContent: 'center', marginTop: space.m },
});
