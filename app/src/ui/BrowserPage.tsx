import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Key } from './Key';

const SIZE = 18; // one text size for every key on this page (matches the More page)

/**
 * The Browser page: six big keys with only words on them (no icons), so nothing can be mistaken
 * for the arrow keys on the main page. Close tab is outlined in red but is a plain tap like the rest.
 */
export function BrowserPage({ send }: { send: (m: Message) => void }) {
  const browser = (action: 'back' | 'forward' | 'reload' | 'prev_tab' | 'next_tab' | 'close_tab') => () => send({ type: 'browser', action });
  return (
    <View style={s.page}>
      <View style={s.row}>
        <Key label="Browser back" text="Back" textSize={SIZE} onPress={browser('back')} />
        <Key label="Browser forward" text="Forward" textSize={SIZE} onPress={browser('forward')} />
      </View>
      <View style={s.row}>
        <Key label="Reload page" text="Reload" textSize={SIZE} onPress={browser('reload')} />
        <Key label="Close tab" text="Close tab" textSize={SIZE} danger onPress={browser('close_tab')} />
      </View>
      <View style={s.row}>
        <Key label="Previous tab" text="Previous tab" textSize={SIZE} onPress={browser('prev_tab')} />
        <Key label="Next tab" text="Next tab" textSize={SIZE} onPress={browser('next_tab')} />
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  page: { flex: 1, gap: space.s },
  row: { flex: 1, flexDirection: 'row', gap: space.s },
});
