import { useEffect, useRef, useState } from 'react';
import { Keyboard, Pressable, StyleSheet, Text, TextInput, View } from 'react-native';
import type { Message } from '../protocol';
import { color, space, type as t } from '../theme';
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

  return (
    <View style={s.bar}>
      <TextInput
        ref={input}
        accessibilityLabel={live ? 'Live typing field' : 'Text to send'}
        style={[s.field, live && s.fieldLive]}
        cursorColor={color.amber}
        selectionColor="rgba(255,180,84,0.35)"
        value={text}
        onChangeText={change}
        onFocus={() => onTyping(true)}
        onBlur={() => onTyping(false)}
        onSubmitEditing={live ? submit : undefined}
        blurOnSubmit={false}
        placeholder={live ? 'Live: typing on your Mac' : 'Type to send'}
        placeholderTextColor={color.textMuted}
        autoCapitalize="none"
        autoCorrect={false}
        returnKeyType={live ? 'send' : 'default'}
        maxLength={500}
      />
      <Pressable
        accessibilityRole="switch"
        accessibilityState={{ checked: live }}
        accessibilityLabel="Live typing"
        onPress={() => {
          tick();
          prev.current = '';
          setText('');
          setLive((v) => !v);
        }}
        style={[s.live, live && s.liveOn]}
      >
        <Text style={[s.liveText, live && { color: color.amber }]}>Live</Text>
      </Pressable>
      {!live ? (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Send text"
          onPress={submit}
          disabled={!text}
          style={[s.send, text ? s.sendReady : null]}
        >
          <Text style={[s.sendText, text ? { color: color.onAmber } : null]}>Send</Text>
        </Pressable>
      ) : null}
    </View>
  );
}

const H = 48; // one height for the field, Live and Send
const s = StyleSheet.create({
  bar: { flexDirection: 'row', gap: space.s, alignItems: 'center' },
  field: { flex: 1, height: H, borderRadius: H / 2, backgroundColor: color.surface, borderWidth: StyleSheet.hairlineWidth * 2, borderColor: 'transparent', paddingHorizontal: space.m, ...t.body },
  fieldLive: { borderColor: color.amber },
  live: { minWidth: 60, height: H, borderRadius: H / 2, alignItems: 'center', justifyContent: 'center', borderWidth: StyleSheet.hairlineWidth * 2, borderColor: color.outline },
  liveOn: { borderColor: color.amber },
  liveText: { ...t.label, fontSize: 13 },
  send: { minWidth: 68, height: H, borderRadius: H / 2, alignItems: 'center', justifyContent: 'center', backgroundColor: color.surface, paddingHorizontal: space.m },
  sendReady: { backgroundColor: color.amber, borderColor: color.amber },
  sendText: { ...t.body, fontSize: 14, color: color.textMuted },
});
