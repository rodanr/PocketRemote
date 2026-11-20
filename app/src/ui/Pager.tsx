import { useRef, useState } from 'react';
import { type LayoutChangeEvent, type NativeScrollEvent, type NativeSyntheticEvent, Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { color } from '../theme';
import { tick } from './haptics';

/**
 * Swipeable pages with small dots in the gap beneath them, so they use no extra height. Every page
 * is as tall as the first and the trackpad never changes size; shorter pages sit centred.
 */
export function Pager({ pages }: { pages: React.ReactNode[] }) {
  const scroller = useRef<React.ComponentRef<typeof ScrollView>>(null);
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);
  const [index, setIndex] = useState(0);

  const settle = (e: NativeSyntheticEvent<NativeScrollEvent>) => {
    if (width > 0) setIndex(Math.round(e.nativeEvent.contentOffset.x / width));
  };
  const go = (i: number) => {
    tick();
    setIndex(i);
    scroller.current?.scrollTo({ x: i * width, animated: true });
  };

  return (
    <View onLayout={(e: LayoutChangeEvent) => setWidth(e.nativeEvent.layout.width)}>
      <ScrollView ref={scroller} horizontal pagingEnabled showsHorizontalScrollIndicator={false} onMomentumScrollEnd={settle} nestedScrollEnabled decelerationRate="fast">
        {pages.map((page, i) => (
          <View key={i} style={{ width, minHeight: height || undefined, justifyContent: 'center' }}>
            {/* Measure the content itself, not the box that carries the minimum height. */}
            <View onLayout={i === 0 ? (e: LayoutChangeEvent) => setHeight(e.nativeEvent.layout.height) : undefined}>{page}</View>
          </View>
        ))}
      </ScrollView>
      <View style={s.dots} accessibilityRole="tablist">
        {pages.map((_, i) => (
          <Pressable key={i} accessibilityRole="tab" accessibilityLabel={`Page ${i + 1} of ${pages.length}`} accessibilityState={{ selected: i === index }} onPress={() => go(i)} hitSlop={14}>
            <View style={[s.dot, i === index && s.dotOn]} />
          </Pressable>
        ))}
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  // Its own row under the keys, so nothing can overlap it.
  dots: { height: 14, marginTop: 4, flexDirection: 'row', justifyContent: 'center', gap: 6, alignItems: 'center' },
  dot: { width: 6, height: 6, borderRadius: 3, backgroundColor: color.rimStrong },
  dotOn: { width: 18, backgroundColor: color.textMuted },
});
