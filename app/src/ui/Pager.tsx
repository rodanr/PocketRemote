import { useState } from 'react';
import { type LayoutChangeEvent, type NativeScrollEvent, type NativeSyntheticEvent, ScrollView, StyleSheet, View } from 'react-native';
import { color } from '../theme';

/**
 * Swipeable pages with tiny dots in the gap beneath them, so they use no extra height. Every page
 * is as tall as the first and the trackpad never changes size; shorter pages sit centred.
 */
export function Pager({ pages }: { pages: React.ReactNode[] }) {
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);
  const [index, setIndex] = useState(0);

  const settle = (e: NativeSyntheticEvent<NativeScrollEvent>) => {
    if (width > 0) setIndex(Math.round(e.nativeEvent.contentOffset.x / width));
  };
  return (
    <View onLayout={(e: LayoutChangeEvent) => setWidth(e.nativeEvent.layout.width)}>
      <ScrollView horizontal pagingEnabled showsHorizontalScrollIndicator={false} onMomentumScrollEnd={settle} nestedScrollEnabled decelerationRate="fast">
        {pages.map((page, i) => (
          <View key={i} style={{ width, minHeight: height || undefined, justifyContent: 'center' }}>
            {/* Measure the content itself, not the box that carries the minimum height. */}
            <View onLayout={i === 0 ? (e: LayoutChangeEvent) => setHeight(e.nativeEvent.layout.height) : undefined}>{page}</View>
          </View>
        ))}
      </ScrollView>
      <View style={s.dots} pointerEvents="none" accessibilityElementsHidden importantForAccessibility="no-hide-descendants">
        {pages.map((_, i) => (
          <View key={i} style={[s.dot, i === index && s.dotOn]} />
        ))}
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  // Centred in the 16 dp gap above the trackpad (8 dp stage gap plus 8 dp margin), so they cost no extra height.
  dots: { position: 'absolute', left: 0, right: 0, bottom: -10, height: 4, flexDirection: 'row', justifyContent: 'center', gap: 5 },
  dot: { width: 4, height: 4, borderRadius: 2, backgroundColor: color.rimStrong },
  dotOn: { width: 14, backgroundColor: color.textMuted },
});
