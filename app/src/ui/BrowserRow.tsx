import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Key } from './Key';

/**
 * 2 x 3 block next to the D-pad: Back/Forward, Reload/Close tab, Previous/Next tab.
 * Close tab is outlined in the danger colour and needs a 300 ms hold: it cannot be undone.
 */
export function BrowserBlock({ send }: { send: (m: Message) => void }) {
  const browser = (action: 'back' | 'forward' | 'reload' | 'prev_tab' | 'next_tab' | 'close_tab') => () => send({ type: 'browser', action });
  return (
    <View style={s.block}>
      <View style={s.row}>
        <Key label="Browser back" icon="arrow-left" height={48} onPress={browser('back')} />
        <Key label="Browser forward" icon="arrow-right" height={48} onPress={browser('forward')} />
      </View>
      <View style={s.row}>
        <Key label="Reload page" icon="reload" height={48} onPress={browser('reload')} />
        <Key label="Close tab" icon="tab-remove" danger holdMs={300} height={48} onPress={browser('close_tab')} />
      </View>
      <View style={s.row}>
        <Key label="Previous tab" icon="chevron-double-left" height={48} onPress={browser('prev_tab')} />
        <Key label="Next tab" icon="chevron-double-right" height={48} onPress={browser('next_tab')} />
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  block: { flex: 1, justifyContent: 'space-between', gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
});
