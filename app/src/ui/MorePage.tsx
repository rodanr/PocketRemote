import { StyleSheet, View } from 'react-native';
import type { Message } from '../protocol';
import { space } from '../theme';
import { Group } from './Group';
import { Key } from './Key';

/**
 * Page 2: three tidy groups of two keys each. Everything here uses actions the Mac already
 * supports reliably; nothing that only works in some apps.
 *  - SLIDES: Page Up / Page Down move through slides in Keynote, PowerPoint, Google Slides and PDFs.
 *  - TRACKS: the Mac's previous / next track keys (whichever app owns "now playing").
 *  - CLICKS: left and right click at the pointer, for when tapping the trackpad is fiddly.
 */
export function MorePage({ send }: { send: (m: Message) => void }) {
  const key = (k: 'page_up' | 'page_down') => () => send({ type: 'key', key: k });
  const media = (action: 'prev' | 'next') => () => send({ type: 'media', action });
  const click = (button: 'left' | 'right') => () => send({ type: 'click', button, count: 1 });
  return (
    <View style={s.page}>
      <Group title="SLIDES" flex={1}>
        <View style={s.row}>
          <Key label="Previous slide" text="Previous slide" textSize={18} height={48} onPress={key('page_up')} />
          <Key label="Next slide" text="Next slide" textSize={18} height={48} onPress={key('page_down')} />
        </View>
      </Group>
      <Group title="TRACKS" flex={1}>
        <View style={s.row}>
          <Key label="Previous track" text="Previous track" textSize={18} height={48} onPress={media('prev')} />
          <Key label="Next track" text="Next track" textSize={18} height={48} onPress={media('next')} />
        </View>
      </Group>
      <Group title="CLICKS" flex={1}>
        <View style={s.row}>
          <Key label="Left click" text="Left click" textSize={18} height={48} onPress={click('left')} />
          <Key label="Right click" text="Right click" textSize={18} height={48} onPress={click('right')} />
        </View>
      </Group>
    </View>
  );
}

const s = StyleSheet.create({
  // Fills the pager's height: the keys stretch to fit, so there are no empty gaps.
  page: { flex: 1, gap: space.s },
  row: { flex: 1, flexDirection: 'row', gap: space.s },
});
