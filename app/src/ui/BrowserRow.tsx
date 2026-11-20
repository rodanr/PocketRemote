import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Group } from './Group';
import { Key } from './Key';

/**
 * Browser keys, each with a word so they cannot be mixed up with the D-pad's arrows:
 * Back / Forward, Reload / Close tab, Previous tab / Next tab.
 * Close tab is outlined in the danger colour and needs a 300 ms hold: it cannot be undone.
 */
export function BrowserBlock({ send }: { send: (m: Message) => void }) {
  const browser = (action: 'back' | 'forward' | 'reload' | 'prev_tab' | 'next_tab' | 'close_tab') => () => send({ type: 'browser', action });
  return (
    <Group title="BROWSER" flex={1}>
      <View style={s.block}>
        <View style={s.row}>
          <Key label="Browser back" icon="arrow-left" text="Back" height={48} onPress={browser('back')} />
          <Key label="Browser forward" icon="arrow-right" iconAfter text="Forward" height={48} onPress={browser('forward')} />
        </View>
        <View style={s.row}>
          <Key label="Reload page" icon="reload" text="Reload" height={48} onPress={browser('reload')} />
          <Key label="Close tab" icon="tab-remove" text="Close tab" danger holdMs={300} height={48} onPress={browser('close_tab')} />
        </View>
        <View style={s.row}>
          <Key label="Previous tab" icon="chevron-left" text="Tab" height={48} onPress={browser('prev_tab')} />
          <Key label="Next tab" icon="chevron-right" iconAfter text="Tab" height={48} onPress={browser('next_tab')} />
        </View>
      </View>
    </Group>
  );
}

const s = StyleSheet.create({
  block: { gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
});
