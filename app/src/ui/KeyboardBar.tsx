import MaterialCommunityIcons from '@expo/vector-icons/MaterialCommunityIcons';
import { useEffect, useRef, useState } from 'react';
import { Keyboard, Pressable, StyleSheet, TextInput, View } from 'react-native';
import type { Message } from '../protocol';
import { color, radius, space, type as t } from '../theme';
import { tick } from './haptics';

interface Props {
  send: (m: Message) => void;
  /** Called when the field gains or loses focus so the screen can collapse the controls. */
  onTyping: (typing: boolean) => void;
}

/**
 * Pill field + Send. In normal mode nothing is sent until you press Send (Enter alone never sends).
 * "Live" mode types every keystroke on the Mac as you go; it is outlined in amber so it is obvious.
 */
export function KeyboardBar({ send, onTyping }: Props) {
  const [text, setText] = useState('');
  const [live, setLive] = useState(false);
  const [focused, setFocused] = useState(false);
  const prev = useRef('');
  const input = useRef<React.ComponentRef<typeof TextInput>>(null);

  // However the keyboard goes away (Back button, swipe, Send), release focus so the controls return.
  useEffect(() => {
    const sub = Keyboard.addListener('keyboardDidHide', () => {
      input.current?.blur();
      onTyping(false);
    });
    return () => sub.remove();
  }, [onTyping]);

  const change = (next: string) => {
    if (live) {
      const before = prev.current;
      let i = 0;
      while (i < before.length && i < next.length && before[i] === next[i]) i++;
      for (let d = 0; d < Math.min(before.length - i, 50); d++) send({ type: 'key', key: 'backspace' });
      const added = next.slice(i);
      if (added) send({ type: 'text', value: added, submit: false });
      prev.current = next;
    }
    setText(next);
  };

  const toggleLive = () => {
    tick();
    prev.current = '';
    setText('');
    setLive((v) => !v);
  };

  const submit = () => {
    if (live) {
      send({ type: 'key', key: 'enter' });
      return;
    }
    if (!text) return;
    tick();
    send({ type: 'text', value: text, submit: true });
    setText('');
    Keyboard.dismiss(); // done typing: bring the controls back
  };

  const hasText = !live && text.length > 0;
  return (
    <View style={s.bar}>
      <View style={s.fieldWrap}>
      <TextInput
        ref={input}
        accessibilityLabel={live ? 'Live typing field' : 'Text to send'}
        style={[s.field, focused && s.fieldOpen, live && s.fieldLive]}
        multiline
        submitBehavior="submit"
        cursorColor={color.amber}
        selectionColor="rgba(255,180,84,0.35)"
        value={text}
        onChangeText={change}
        onFocus={() => {
          setFocused(true);
          onTyping(true);
        }}
        onBlur={() => {
          setFocused(false);
          onTyping(false);
        }}
        onSubmitEditing={live ? submit : undefined}
        placeholder={live ? 'Live: typing on your computer' : 'Type to send'}
        placeholderTextColor={color.textMuted}
        autoCapitalize="none"
        autoCorrect={false}
        returnKeyType={live ? 'send' : 'default'}
        maxLength={500}
      />
      {focused ? (
        <Pressable accessibilityRole="button" accessibilityLabel="Hide keyboard" onPress={() => Keyboard.dismiss()} hitSlop={6} style={s.hide}>
          <View style={s.hideDot}>
            <MaterialCommunityIcons name="keyboard-close-outline" size={18} color={color.text} />
          </View>
        </Pressable>
      ) : null}
      </View>
      {/* Backspace and Enter always act on the computer (to fix or submit its own text boxes). The last slot is Send once you have typed text, otherwise the Live switch. */}
      <Pressable accessibilityRole="button" accessibilityLabel="Backspace" onPress={() => { tick(); send({ type: 'key', key: 'backspace' }); }} style={s.round}>
        <MaterialCommunityIcons name="backspace-outline" size={22} color={color.text} />
      </Pressable>
      <Pressable accessibilityRole="button" accessibilityLabel="Enter" onPress={() => { tick(); send({ type: 'key', key: 'enter' }); }} style={s.round}>
        <MaterialCommunityIcons name="keyboard-return" size={22} color={color.text} />
      </Pressable>
      {hasText ? (
        <Pressable accessibilityRole="button" accessibilityLabel="Send text" onPress={submit} style={[s.round, s.sendReady]}>
          <MaterialCommunityIcons name="send" size={22} color={color.onAmber} />
        </Pressable>
      ) : (
        <Pressable accessibilityRole="switch" accessibilityState={{ checked: live }} accessibilityLabel="Live typing" onPress={toggleLive} style={[s.round, live && s.liveOn]}>
          <MaterialCommunityIcons name="lightning-bolt" size={22} color={live ? color.amber : color.textMuted} />
        </Pressable>
      )}
    </View>
  );
}

const H = 48; // one height for the field, Live and Send
const s = StyleSheet.create({
  bar: { flexDirection: 'row', gap: space.s, alignItems: 'flex-start' },
  fieldWrap: { flex: 1 },
  // One line when idle; a fixed four-line height while typing (it scrolls inside), so it never resizes as you type.
  field: { height: H, borderRadius: radius.m, paddingLeft: space.m, paddingRight: space.m, paddingVertical: 12, backgroundColor: color.surface, borderWidth: 1, borderColor: color.rim, textAlignVertical: 'center', ...t.body },
  fieldOpen: { height: 104, paddingRight: 52, textAlignVertical: 'top' },
  fieldLive: { borderColor: color.amber },
  hide: { position: 'absolute', right: 8, top: 8, width: 36, height: 36, alignItems: 'center', justifyContent: 'center' },
  // A clear filled circle: the app's own way out of typing, so the Back key is never needed.
  hideDot: { width: 32, height: 32, borderRadius: 10, backgroundColor: color.surfacePressed, borderWidth: 1, borderColor: color.rimStrong, alignItems: 'center', justifyContent: 'center' },
  round: { width: H, height: H, borderRadius: radius.m, alignItems: 'center', justifyContent: 'center', backgroundColor: color.surface, borderWidth: 1, borderColor: color.rim },
  liveOn: { borderColor: color.amber },
  sendReady: { backgroundColor: color.amber, borderColor: 'rgba(255,255,255,0.18)' },
});
