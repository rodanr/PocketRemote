import { useState } from 'react';
import { type LayoutChangeEvent, type NativeScrollEvent, type NativeSyntheticEvent, ScrollView, StyleSheet, View } from 'react-native';
import { color, space } from '../theme';

/**
 * Two (or more) horizontally paged pages with a dot indicator. Every page is as tall as the
 * first one, so switching pages never makes the trackpad below jump.
 */
export function Pager({ pages }: { pages: React.ReactNode[] }) {
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);
  const [index, setIndex] = useState(0);

  const settle = (e: NativeSyntheticEvent<NativeScrollEvent>) => {
    if (width > 0) setIndex(Math.round(e.nativeEvent.contentOffset.x / width));
  };

  return (
    <View onLayout={(e: LayoutChangeEvent) => setWidth(e.nativeEvent.layout.width)} accessibilityLabel={`Page ${index + 1} of ${pages.length}`}>
      <ScrollView
        horizontal
        pagingEnabled
        showsHorizontalScrollIndicator={false}
        onMomentumScrollEnd={settle}
        scrollEventThrottle={32}
        nestedScrollEnabled
        // Do not let a half-swipe leave the pager between pages.
        decelerationRate="fast"
      >
        {pages.map((page, i) => (
          <View key={i} style={{ width, minHeight: height || undefined }}>
            {/* Measure the content itself (not the box that carries the minimum height), or the
                height could only ever grow. */}
            <View
              style={i > 0 ? s.fill : undefined}
              onLayout={i === 0 ? (e: LayoutChangeEvent) => setHeight(e.nativeEvent.layout.height) : undefined}
            >
              {page}
            </View>
          </View>
        ))}
      </ScrollView>
      <View style={s.dots} accessible={false}>
        {pages.map((_, i) => (
          <View key={i} style={[s.dot, i === index && s.dotActive]} />
        ))}
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  fill: { flex: 1 },
  // 6 dp dots centred in the 8 dp gap above the trackpad, so they cost no height.
  dots: { position: 'absolute', left: 0, right: 0, bottom: -7, flexDirection: 'row', justifyContent: 'center', gap: space.s },
  dot: { width: 6, height: 6, borderRadius: 3, backgroundColor: color.hairline },
  dotActive: { backgroundColor: color.textMuted },
});
