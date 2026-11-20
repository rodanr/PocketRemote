import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Dpad } from './Dpad';
import { Key } from './Key';

/**
 * The Main page, icons only. Play/Pause is the one amber key and is twice as wide, so a seek or
 * volume key cannot be hit by mistake. Captions and Fullscreen sit to the left of the arrows,
 * Mute and Esc to the right.
 */
export function MainPage({ send, active }: { send: (m: Message) => void; active: boolean }) {
  const media = (action: 'play_pause' | 'mute' | 'vol_up' | 'vol_down') => () => send({ type: 'media', action });
  const player = (action: 'seek_back' | 'seek_fwd' | 'fullscreen' | 'captions') => () => send({ type: 'player', action });
  return (
    <View style={s.page}>
      <View style={s.row}>
        <Key label="Play or pause" icon="play-pause" iconSize={28} lead={active} flex={2} onPress={media('play_pause')} />
        <Key label="Back 10 seconds" icon="rewind-10" onPress={player('seek_back')} />
        <Key label="Forward 10 seconds" icon="fast-forward-10" onPress={player('seek_fwd')} />
      </View>
      <View style={s.row}>
        <Key label="Volume down" icon="volume-minus" repeat onPress={media('vol_down')} />
        <Key label="Volume up" icon="volume-plus" repeat onPress={media('vol_up')} />
      </View>
      <View style={s.padRow}>
        <View style={s.side}>
          <Key label="Captions" icon="closed-caption" flex={0} onPress={player('captions')} />
          <Key label="Fullscreen" icon="fullscreen" flex={0} onPress={player('fullscreen')} />
        </View>
        <Dpad send={send} />
        <View style={s.side}>
          <Key label="Mute" icon="volume-off" flex={0} onPress={media('mute')} />
          <Key label="Escape" icon="keyboard-esc" flex={0} onPress={() => send({ type: 'key', key: 'escape' })} />
        </View>
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  page: { gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
  padRow: { flexDirection: 'row', alignItems: 'center', gap: space.s },
  side: { flex: 1, gap: space.s, justifyContent: 'center' },
});
