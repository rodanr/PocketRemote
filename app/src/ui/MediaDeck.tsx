import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Key } from './Key';

/**
 * [Play (double width)][-10][+10] / [Mute][Vol -][Vol +] / [CC][Full]
 * Play/Pause is the one amber control. Seek sits away from Fullscreen and Mute away from Vol -,
 * so a thumb that is not watching cannot trigger the wrong one.
 */
export function MediaDeck({ send }: { send: (m: Message) => void }) {
  const media = (action: 'play_pause' | 'mute' | 'vol_up' | 'vol_down') => () => send({ type: 'media', action });
  const player = (action: 'seek_back' | 'seek_fwd' | 'fullscreen' | 'captions') => () => send({ type: 'player', action });
  return (
    <View style={s.deck}>
      <View style={s.row}>
        <Key label="Play or pause" icon="play-pause" lead flex={2} onPress={media('play_pause')} />
        <Key label="Back 10 seconds" icon="rewind-10" onPress={player('seek_back')} />
        <Key label="Forward 10 seconds" icon="fast-forward-10" onPress={player('seek_fwd')} />
      </View>
      <View style={s.row}>
        <Key label="Mute" icon="volume-off" onPress={media('mute')} />
        <Key label="Volume down" icon="volume-minus" repeat onPress={media('vol_down')} />
        <Key label="Volume up" icon="volume-plus" repeat onPress={media('vol_up')} />
      </View>
      <View style={s.row}>
        <Key label="Captions" icon="closed-caption" onPress={player('captions')} />
        <Key label="Fullscreen" icon="fullscreen" onPress={player('fullscreen')} />
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  deck: { gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
});
