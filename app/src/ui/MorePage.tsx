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
      <Group title="SLIDES">
        <View style={s.row}>
          <Key label="Previous slide" icon="page-previous-outline" text="Previous slide" height={76} onPress={key('page_up')} />
          <Key label="Next slide" icon="page-next-outline" iconAfter text="Next slide" height={76} onPress={key('page_down')} />
        </View>
      </Group>
      <Group title="TRACKS">
        <View style={s.row}>
          <Key label="Previous track" icon="skip-previous" text="Previous track" height={60} onPress={media('prev')} />
          <Key label="Next track" icon="skip-next" iconAfter text="Next track" height={60} onPress={media('next')} />
        </View>
      </Group>
      <Group title="CLICKS">
        <View style={s.row}>
          <Key label="Left click" icon="mouse-left-click-outline" text="Left click" height={60} onPress={click('left')} />
          <Key label="Right click" icon="mouse-right-click-outline" text="Right click" height={60} onPress={click('right')} />
        </View>
      </Group>
    </View>
  );
}

const s = StyleSheet.create({
  // Fills the pager's height and spreads the groups evenly, so the page never looks empty.
  page: { flex: 1, justifyContent: 'space-between', gap: space.s },
  row: { flexDirection: 'row', gap: space.s },
});
