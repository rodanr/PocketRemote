import { Pressable, StyleSheet, Text, View } from 'react-native';
import type { ConnState } from '../connection';
import { color, space, TARGET, type as t } from '../theme';
import { describe } from './StatusHeader';

interface Props {
  state: ConnState;
  onReconnect: () => void;
  onRescan: () => void;
}

/**
 * Shown over the controls (below the header) when there is no connection, so nothing gets pushed
 * down. The header stays visible, so Settings and "forget this computer" remain reachable.
 */
export function ConnectionDialog({ state, onReconnect, onRescan }: Props) {
  if (state.phase === 'connected' || state.phase === 'connecting') return null;
  const d = describe(state);
  const paused = d.kind === 'paused';
  return (
    <View style={s.scrim} accessibilityLiveRegion="polite">
      <View style={s.card}>
        <Text style={t.title} accessibilityRole="header">
          {paused ? 'Paused on your computer' : d.word}
        </Text>
        <Text style={[t.body, s.body]}>{paused ? 'Resume it there to keep going.' : (d.hint ?? '')}</Text>
        {state.reason === 'paused' ? null : (
          <Pressable accessibilityRole="button" onPress={state.needsRescan ? onRescan : onReconnect} style={s.button}>
            <Text style={s.buttonText}>{state.needsRescan ? 'Scan again' : 'Reconnect'}</Text>
          </Pressable>
        )}
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  scrim: { position: 'absolute', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(7,7,10,0.78)', alignItems: 'center', justifyContent: 'center', padding: space.l },
  card: { width: '100%', maxWidth: 360, borderRadius: 24, padding: space.l, backgroundColor: color.groove, borderWidth: 1, borderColor: color.rim, gap: space.s },
  body: { color: color.textMuted },
  button: { minHeight: TARGET, borderRadius: TARGET / 2, alignItems: 'center', justifyContent: 'center', backgroundColor: color.amber, marginTop: space.s },
  buttonText: { color: color.onAmber, fontSize: 15, fontWeight: '600', letterSpacing: 0.3 },
});
