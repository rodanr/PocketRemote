import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Group } from './Group';
import { Key } from './Key';

const SIZE = 15; // one text size for every key on this page

/**
 * "Browser & more": three titled sections, words only, so nothing is mistaken for the arrow keys.
 *  - BROWSER: history, reload and tabs. Close tab is red-tinted but a plain tap.
 *  - SLIDES: Page Up / Page Down move through slides in Keynote, PowerPoint, Google Slides, PDFs.
 *  - TRACKS: the Mac's previous / next track keys (whichever app owns "now playing").
 */
export function MorePage({ send }: { send: (m: Message) => void }) {
  const browser = (action: 'back' | 'forward' | 'reload' | 'prev_tab' | 'next_tab' | 'close_tab') => () => send({ type: 'browser', action });
  const key = (k: 'page_up' | 'page_down') => () => send({ type: 'key', key: k });
  const media = (action: 'prev' | 'next') => () => send({ type: 'media', action });
  return (
    <View style={s.page}>
      <Group title="BROWSER">
        <View style={s.grid}>
          <View style={s.row}>
            <Key label="Browser back" text="Back" textSize={SIZE} onPress={browser('back')} />
            <Key label="Browser forward" text="Forward" textSize={SIZE} onPress={browser('forward')} />
            <Key label="Reload page" text="Reload" textSize={SIZE} onPress={browser('reload')} />
          </View>
          <View style={s.row}>
            <Key label="Previous tab" text="Prev tab" textSize={SIZE} onPress={browser('prev_tab')} />
            <Key label="Next tab" text="Next tab" textSize={SIZE} onPress={browser('next_tab')} />
            <Key label="Close tab" text="Close tab" textSize={SIZE} danger onPress={browser('close_tab')} />
          </View>
        </View>
      </Group>
      <Group title="SLIDES">
        <View style={s.row}>
          <Key label="Previous slide" text="Previous slide" textSize={SIZE} onPress={key('page_up')} />
          <Key label="Next slide" text="Next slide" textSize={SIZE} onPress={key('page_down')} />
        </View>
      </Group>
      <Group title="TRACKS">
        <View style={s.row}>
          <Key label="Previous track" text="Previous track" textSize={SIZE} onPress={media('prev')} />
          <Key label="Next track" text="Next track" textSize={SIZE} onPress={media('next')} />
        </View>
      </Group>
    </View>
  );
}

const s = StyleSheet.create({
  page: { gap: space.s },
  grid: { gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
});
