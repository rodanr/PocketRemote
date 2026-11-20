import { Pressable, StyleSheet, View } from 'react-native';
import type { ConnState } from '../connection';
import { color, space, TARGET, type as t } from '../theme';
import { describe } from './StatusHeader';
import { Text } from './Text';

interface Props {
  state: ConnState;
  onReconnect: () => void;
  onRescan: () => void;
}

/**
 * Shown over the controls (below the header) when there is no connection, so nothing gets pushed
 * down. The header stays visible, so Settings and "forget this computer" remain reachable.
 * It stays up while a retry is in progress (no flashing); when pairing looks broken it offers
 * "Scan again" first and keeps a quiet "Try again" beside it.
 */
export function ConnectionDialog({ state, onReconnect, onRescan }: Props) {
  if (state.phase === 'connected') return null;
  const retrying = state.phase === 'connecting';
  // The very first connect after opening the app is silent; a retry after a failure keeps the dialog steady.
  if (retrying && state.attempt === 0 && !state.needsRescan) return null;
  const d = describe(state);
  const paused = d.kind === 'paused';
  const title = retrying ? 'Trying again…' : paused ? 'Paused on your computer' : d.word;
  const hint = retrying ? 'This can take a few seconds.' : paused ? 'Resume it there to keep going.' : (d.hint ?? '');
  return (
    <View style={s.scrim} accessibilityLiveRegion="polite">
      <View style={s.card}>
        <Text style={t.title} accessibilityRole="header">
          {title}
        </Text>
        <Text style={[t.body, s.body]}>{hint}</Text>
        {state.reason === 'paused' ? null : state.needsRescan ? (
          <>
            <Pressable accessibilityRole="button" onPress={onRescan} style={s.button}>
              <Text style={s.buttonText}>Scan again</Text>
            </Pressable>
            <Pressable accessibilityRole="button" accessibilityState={{ disabled: retrying }} disabled={retrying} onPress={onReconnect} style={s.quiet}>
              <Text style={s.quietText}>{retrying ? 'Trying…' : 'Try again'}</Text>
            </Pressable>
          </>
        ) : (
          <Pressable accessibilityRole="button" accessibilityState={{ disabled: retrying }} disabled={retrying} onPress={onReconnect} style={[s.button, retrying && s.disabled]}>
            <Text style={s.buttonText}>{retrying ? 'Trying…' : 'Reconnect'}</Text>
          </Pressable>
        )}
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  scrim: { position: 'absolute', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(20,18,16,0.82)', alignItems: 'center', justifyContent: 'center', padding: space.l },
  card: { width: '100%', maxWidth: 360, borderRadius: 16, padding: space.l, backgroundColor: color.panel, borderWidth: 1, borderColor: color.hairline, gap: space.s },
  body: { color: color.textMuted },
  button: { minHeight: TARGET, borderRadius: 16, alignItems: 'center', justifyContent: 'center', backgroundColor: color.amber, marginTop: space.s },
  disabled: { opacity: 0.6 },
  buttonText: { color: color.onAmber, fontSize: 15, fontWeight: '600', letterSpacing: 0.3 },
  quiet: { minHeight: TARGET, alignItems: 'center', justifyContent: 'center' },
  quietText: { color: color.textMuted, fontSize: 14, fontWeight: '500' },
});
