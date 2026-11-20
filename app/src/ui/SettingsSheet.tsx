import { Modal, Pressable, StyleSheet, Switch, Text, View } from 'react-native';
import type { PointerSettings } from '../trackpad/pointer';
import { color, radius, space, TARGET, type as t } from '../theme';
import { tick } from './haptics';

interface Props {
  visible: boolean;
  settings: PointerSettings;
  hostName: string;
  onChange: (s: PointerSettings) => void;
  onClose: () => void;
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

export function SettingsSheet({ visible, settings, hostName, onChange, onClose, onForget }: Props) {
  return (
    <Modal visible={visible} transparent animationType="slide" onRequestClose={onClose} statusBarTranslucent>
      <Pressable style={s.scrim} onPress={onClose} accessibilityLabel="Close settings" />
      <View style={s.sheet}>
        <View style={s.grab} />
        <Text style={[t.title, { marginBottom: space.m }]}>Settings</Text>
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
      </View>
    </Modal>
  );
}

const s = StyleSheet.create({
  scrim: { flex: 1, backgroundColor: 'rgba(0,0,0,0.55)' },
  sheet: { backgroundColor: color.surface, borderTopLeftRadius: radius.pill, borderTopRightRadius: radius.pill, padding: space.l, paddingBottom: space.l + 8, borderTopWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline },
  grab: { alignSelf: 'center', width: 36, height: 4, borderRadius: 2, backgroundColor: color.outline, marginBottom: space.m },
  row: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', minHeight: 56 },
  stepper: { flexDirection: 'row', alignItems: 'center' },
  step: { width: TARGET, height: TARGET, borderRadius: TARGET / 2, alignItems: 'center', justifyContent: 'center', borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline },
  stepText: { color: color.text, fontSize: 22 },
  gesture: { flexDirection: 'row', justifyContent: 'space-between', paddingVertical: 6 },
  how: { ...t.body, fontSize: 14, color: color.text },
  what: { ...t.body, fontSize: 14, color: color.textMuted },
  forget: { minHeight: TARGET, justifyContent: 'center', marginTop: space.m },
});
