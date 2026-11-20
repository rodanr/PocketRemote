import { useEffect, useRef, useState } from 'react';
import { Animated, Easing, type LayoutChangeEvent, type NativeScrollEvent, type NativeSyntheticEvent, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { color, space } from '../theme';
import { tick } from './haptics';

const TAB_H = 28;

/**
 * Swipeable pages with a named tab strip beneath them, so people can see what is on each page
 * (the strip is the only thing that tells them the Browser keys exist). Every page is as tall as
 * the first, so switching never makes the trackpad below jump.
 */
export function Pager({ pages, labels }: { pages: React.ReactNode[]; labels: string[] }) {
  const scroller = useRef<React.ComponentRef<typeof ScrollView>>(null);
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);
  const [tabsWidth, setTabsWidth] = useState(0);
  const [index, setIndex] = useState(0);
  const [slide] = useState(() => new Animated.Value(0));

  useEffect(() => {
    Animated.timing(slide, { toValue: index, duration: 180, easing: Easing.out(Easing.cubic), useNativeDriver: true }).start();
  }, [index, slide]);

  const settle = (e: NativeSyntheticEvent<NativeScrollEvent>) => {
    if (width > 0) setIndex(Math.round(e.nativeEvent.contentOffset.x / width));
  };
  const go = (i: number) => {
    tick();
    setIndex(i);
    scroller.current?.scrollTo({ x: i * width, animated: true });
  };

  const seg = tabsWidth / labels.length;
  return (
    <View onLayout={(e: LayoutChangeEvent) => setWidth(e.nativeEvent.layout.width)}>
      <ScrollView
        ref={scroller}
        horizontal
        pagingEnabled
        showsHorizontalScrollIndicator={false}
        onMomentumScrollEnd={settle}
        scrollEventThrottle={32}
        nestedScrollEnabled
        decelerationRate="fast"
      >
        {pages.map((page, i) => (
          <View key={i} style={{ width, minHeight: height || undefined }}>
            {/* Measure the content itself (not the box that carries the minimum height). */}
            <View onLayout={i === 0 ? (e: LayoutChangeEvent) => setHeight(e.nativeEvent.layout.height) : undefined}>{page}</View>
          </View>
        ))}
      </ScrollView>
      <View style={s.tabs} onLayout={(e: LayoutChangeEvent) => setTabsWidth(e.nativeEvent.layout.width)} accessibilityRole="tablist">
        {seg > 0 ? (
          <Animated.View
            pointerEvents="none"
            style={[s.pill, { width: seg - 6, transform: [{ translateX: Animated.add(Animated.multiply(slide, seg), 3) }] }]}
          />
        ) : null}
        {labels.map((l, i) => (
          <Pressable
            key={l}
            accessibilityRole="tab"
            accessibilityState={{ selected: i === index }}
            onPress={() => go(i)}
            hitSlop={{ top: 10, bottom: 10 }}
            style={s.tab}
          >
            <Text style={[s.label, i === index && s.labelActive]} maxFontSizeMultiplier={1.4} numberOfLines={1}>
              {l}
            </Text>
          </Pressable>
        ))}
      </View>
    </View>
  );
}

const s = StyleSheet.create({
  tabs: { flexDirection: 'row', height: TAB_H, marginTop: space.s, alignItems: 'center' },
  // The sliding highlight behind the active tab: a soft raised pill, not amber (amber is for Play).
  pill: { position: 'absolute', left: 0, top: 0, height: TAB_H, borderRadius: TAB_H / 2, backgroundColor: color.surface, borderWidth: 1, borderColor: color.rim },
  tab: { flex: 1, height: TAB_H, alignItems: 'center', justifyContent: 'center' },
  label: { fontSize: 13, fontWeight: '500', letterSpacing: 0.3, color: color.textMuted },
  labelActive: { color: color.text },
});
