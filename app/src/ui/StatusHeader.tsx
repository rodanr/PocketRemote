import MaterialCommunityIcons from '@expo/vector-icons/MaterialCommunityIcons';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import type { ConnState, Reason } from '../connection';
import { color, space, TARGET, type as t } from '../theme';

/** Words for every state: status is never conveyed by colour alone. */
export function describe(state: ConnState): { word: string; kind: 'ok' | 'busy' | 'paused' | 'down'; hint?: string } {
  if (state.phase === 'connected') {
    return state.status?.paused ? { word: 'Paused', kind: 'paused', hint: 'The Mac paused the remote' } : { word: 'Connected', kind: 'ok' };
  }
  if (state.phase === 'connecting') return { word: 'Connecting…', kind: 'busy' };
  const why: Record<Reason, { word: string; kind: 'paused' | 'down'; hint: string }> = {
    unreachable: { word: "Can't reach your Mac", kind: 'down', hint: 'Is it awake and on the same Wi-Fi?' },
    lost: { word: 'Connection lost', kind: 'down', hint: 'Trying again…' },
    paused: { word: 'Paused', kind: 'paused', hint: 'Resume it from the Mac' },
    rate_limited: { word: 'Slow down', kind: 'down', hint: 'Too many commands at once' },
    revoked: { word: 'Phone removed', kind: 'down', hint: 'Pair again to continue' },
    replaced: { word: 'Another device took over', kind: 'down', hint: 'Tap to reconnect' },
    idle: { word: 'Disconnected (idle)', kind: 'down', hint: 'Tap to reconnect' },
  };
  return why[state.reason ?? 'lost'] ?? { word: 'Disconnected', kind: 'down' };
}

/** Shape plus words: solid dot = connected, hollow ring = connecting, ring with bars = paused, struck ring = down. */
function Cue({ kind }: { kind: 'ok' | 'busy' | 'paused' | 'down' }) {
  if (kind === 'ok') return <View style={[c.dot, { backgroundColor: color.ok }]} />;
  if (kind === 'busy') return <View style={[c.ring, { borderColor: color.textMuted, borderStyle: 'dashed' }]} />;
  if (kind === 'paused') {
    return (
      <View style={[c.ring, { borderColor: color.danger, flexDirection: 'row', gap: 2 }]}>
        <View style={c.bar} />
        <View style={c.bar} />
      </View>
    );
  }
  return (
    <View style={[c.ring, { borderColor: color.danger }]}>
      <View style={c.strike} />
    </View>
  );
}

interface Props {
  state: ConnState;
  hostName: string;
  profile: string;
  onReconnect: () => void;
  onSettings: () => void;
  onRescan: () => void;
}

export function StatusHeader({ state, hostName, profile, onReconnect, onSettings, onRescan }: Props) {
  const d = describe(state);
  const connected = state.phase === 'connected';
  return (
    <View>
      <View style={s.header}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`${d.word}. ${hostName}.${connected ? '' : ' Tap to reconnect.'}`}
          onPress={connected ? undefined : onReconnect}
          style={s.status}
        >
          <Cue kind={d.kind} />
          <View style={{ flexShrink: 1 }}>
            <Text style={t.title} numberOfLines={1}>
              {hostName}
            </Text>
            <Text style={[t.label, d.kind === 'down' && { color: color.danger }]} numberOfLines={1}>
              {d.word}
            </Text>
          </View>
        </Pressable>
        <View style={s.chip} accessibilityLabel={`Player profile ${profile}`}>
          <Text style={s.chipText}>{profile}</Text>
        </View>
        <Pressable accessibilityRole="button" accessibilityLabel="Settings" onPress={onSettings} style={s.icon}>
          <MaterialCommunityIcons name="cog-outline" size={24} color={color.text} />
        </Pressable>
      </View>
      {!connected && state.phase !== 'connecting' ? (
        <View style={s.banner} accessibilityLiveRegion="polite">
          <Text style={[t.body, { flex: 1 }]}>{d.hint ?? d.word}</Text>
          <Pressable accessibilityRole="button" onPress={state.needsRescan ? onRescan : onReconnect} style={s.bannerBtn}>
            <Text style={s.bannerBtnText}>{state.needsRescan ? 'Scan again' : 'Reconnect'}</Text>
          </Pressable>
        </View>
      ) : null}
    </View>
  );
}

const c = StyleSheet.create({
  dot: { width: 14, height: 14, borderRadius: 7 },
  ring: { width: 16, height: 16, borderRadius: 8, borderWidth: 2, alignItems: 'center', justifyContent: 'center' },
  bar: { width: 2, height: 6, backgroundColor: color.danger },
  strike: { width: 12, height: 2, backgroundColor: color.danger, transform: [{ rotate: '-45deg' }] },
});

const s = StyleSheet.create({
  header: { flexDirection: 'row', alignItems: 'center', gap: space.s, minHeight: 56 },
  status: { flex: 1, flexDirection: 'row', alignItems: 'center', gap: space.s + 4, minHeight: TARGET },
  chip: { minHeight: 36, paddingHorizontal: space.m - 4, borderRadius: 18, justifyContent: 'center', backgroundColor: color.surface, borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline },
  chipText: { ...t.body, fontSize: 13, textTransform: 'capitalize' },
  icon: { width: TARGET, height: TARGET, alignItems: 'center', justifyContent: 'center' },
  banner: { flexDirection: 'row', alignItems: 'center', gap: space.s, padding: space.s + 4, borderRadius: 14, backgroundColor: color.surface, borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline, marginBottom: space.s },
  bannerBtn: { minHeight: 40, paddingHorizontal: space.m, justifyContent: 'center', borderRadius: 20, backgroundColor: color.amber },
  bannerBtnText: { color: color.onAmber, fontWeight: '500', fontSize: 14 },
});
