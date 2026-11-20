import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Key } from './Key';

/** 2 x 3 block next to the D-pad: Back/Forward, Reload/Esc, Previous/Next tab. Close tab lives on the trackpad corner. */
export function BrowserBlock({ send }: { send: (m: Message) => void }) {
  const browser = (action: 'back' | 'forward' | 'reload' | 'prev_tab' | 'next_tab') => () => send({ type: 'browser', action });
  return (
    <View style={s.block}>
      <View style={s.row}>
        <Key label="Browser back" icon="arrow-left" height={48} onPress={browser('back')} />
        <Key label="Browser forward" icon="arrow-right" height={48} onPress={browser('forward')} />
      </View>
      <View style={s.row}>
        <Key label="Reload page" icon="reload" height={48} onPress={browser('reload')} />
        <Key label="Escape" text="Esc" height={48} onPress={() => send({ type: 'key', key: 'escape' })} />
      </View>
      <View style={s.row}>
        <Key label="Previous tab" icon="chevron-left" text="Tab" height={48} onPress={browser('prev_tab')} />
        <Key label="Next tab" icon="chevron-right" text="Tab" height={48} onPress={browser('next_tab')} />
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  block: { flex: 1, justifyContent: 'space-between', gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
});
