import { CameraView, useCameraPermissions } from 'expo-camera';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Animated, BackHandler, Easing, Linking, Pressable, StyleSheet, TextInput, View } from 'react-native';
import { Text } from './Text';
import type { Keypair } from '../noise';
import { secureRng } from '../rng';
import { PairingClient, parsePairingUri, type PairingEvent, type PairingOffer } from '../session';
import { savePairing, type Pairing } from '../storage';
import { color, font, radius, space, TARGET, type as t } from '../theme';
import { press, tick } from './haptics';

type Step = 'intro' | 'scan' | 'working' | 'code' | 'error';

const PAIRING_TIMEOUT_MS = 130_000; // the host's window is 120 s

const RING = 288;
const RING_DOTS = 14;

/** A ring of amber lights framing the pairing code (the computer's QR gets the same frame). */
function Ring({ children }: { children: React.ReactNode }) {
  return (
    <View style={{ width: RING, height: RING, alignItems: 'center', justifyContent: 'center' }}>
      {Array.from({ length: RING_DOTS }, (_, i) => {
        const a = (i / RING_DOTS) * Math.PI * 2 - Math.PI / 2;
        const r = RING / 2 - 6;
        return (
          <View
            key={i}
            style={{ position: 'absolute', width: 6, height: 6, borderRadius: 3, backgroundColor: color.amber, opacity: 0.75, left: RING / 2 + r * Math.cos(a) - 3, top: RING / 2 + r * Math.sin(a) - 3 }}
          />
        );
      })}
      {children}
    </View>
  );
}

/** The aisle-light runway as an empty-state graphic: a dim column of lights leading to one lit point. */
function Runway() {
  const dots = [10, 14, 20, 28, 38];
  return (
    <View style={s.runway} accessible={false}>
      {dots.map((d, i) => (
        <View key={i} style={{ width: d, height: d, borderRadius: d / 2, backgroundColor: color.amber, opacity: 0.25 + i * 0.1 }} />
      ))}
    </View>
  );
}

export function PairingScreen({ phone, onPaired }: { phone: Keypair; onPaired: (p: Pairing) => void }) {
  const [step, setStep] = useState<Step>('intro');
  const [code, setCode] = useState('');
  const [addr, setAddr] = useState('');
  const [countdown] = useState(() => new Animated.Value(1));
  const [error, setError] = useState('');
  const [permission, requestPermission] = useCameraPermissions();
  const socket = useRef<WebSocket | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const handled = useRef(false);

  const cleanup = useCallback(() => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    const ws = socket.current;
    socket.current = null;
    if (ws) {
      ws.onopen = ws.onmessage = ws.onclose = ws.onerror = null;
      try {
        ws.close();
      } catch {
        // already closed
      }
    }
  }, []);

  useEffect(() => cleanup, [cleanup]);

  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      if (step === 'intro') return false;
      cleanup();
      setStep('intro');
      return true;
    });
    return () => sub.remove();
  }, [step, cleanup]);

  const fail = useCallback(
    (msg: string) => {
      cleanup();
      setError(msg);
      setStep('error');
    },
    [cleanup],
  );

  const begin = useCallback(
    (offer: PairingOffer) => {
      setAddr(offer.addr);
      setStep('working');
      const ws = new WebSocket(`ws://${offer.addr}/ws`);
      ws.binaryType = 'arraybuffer';
      socket.current = ws;
      const client: PairingClient = new PairingClient({
        rng: secureRng,
        phone,
        offer,
        send: (f) => ws.send(f),
        onEvent: (e: PairingEvent) => {
          if (e.kind === 'code') {
            press();
            setCode(e.code);
            setStep('code');
            countdown.setValue(1);
            Animated.timing(countdown, { toValue: 0, duration: 120_000, easing: Easing.linear, useNativeDriver: true }).start();
          } else if (e.kind === 'paired') {
            const pairing: Pairing = { addr: offer.addr, hostKey: e.hostKey, hostId: offer.hostId, hostName: e.hostName };
            void savePairing(pairing).then(() => {
              cleanup();
              onPaired(pairing);
            });
          } else {
            fail("Pairing didn't complete. The computer may have rejected it or the code expired.");
          }
        },
      });
      ws.onopen = () => client.start();
      ws.onmessage = (ev) => client.receive(new Uint8Array(ev.data as ArrayBuffer));
      ws.onclose = () => client.remoteClosed();
      ws.onerror = () => undefined;
      timer.current = setTimeout(() => fail('That took too long. Start pairing again on your computer.'), PAIRING_TIMEOUT_MS);
    },
    [phone, onPaired, cleanup, fail, countdown],
  );

  const onScan = useCallback(
    (data: string) => {
      if (handled.current) return;
      const offer = parsePairingUri(data);
      handled.current = true;
      if (!offer) {
        fail("That isn't a PocketRemote code. Scan the code shown on your computer.");
        return;
      }
      tick();
      begin(offer);
    },
    [begin, fail],
  );

  const startScan = async () => {
    handled.current = false;
    if (!permission?.granted) {
      const r = await requestPermission();
      if (!r.granted) {
        setError(r.canAskAgain ? 'PocketRemote needs the camera to read the code on your computer.' : 'Camera access is turned off. Enable it in Settings to scan the code.');
        setStep('error');
        return;
      }
    }
    setStep('scan');
  };

  if (step === 'scan') {
    return (
      <View style={s.fill}>
        <CameraView
          style={StyleSheet.absoluteFill}
          facing="back"
          barcodeScannerSettings={{ barcodeTypes: ['qr'] }}
          onBarcodeScanned={({ data }) => onScan(data)}
        />
        <View style={s.scrim} pointerEvents="none" />
        <View style={s.frame} pointerEvents="none">
          {[s.cTL, s.cTR, s.cBL, s.cBR].map((c, i) => (
            <View key={i} style={[s.corner, c]} />
          ))}
        </View>
        <View style={s.scanBottom}>
          <Text style={[t.body, { textAlign: 'center' }]}>Point at the code on your computer</Text>
          <Pressable accessibilityRole="button" onPress={() => setStep('intro')} style={s.ghost}>
            <Text style={t.body}>Cancel</Text>
          </Pressable>
        </View>
      </View>
    );
  }

  if (step === 'intro') {
    return (
      <View style={s.fill}>
        <View style={s.introTop}>
          <Runway />
          <Text style={[t.title, s.h]}>Scan the code on your computer</Text>
          <Text style={[t.body, s.p]}>Open PocketRemote on your computer and choose Pair phone. Both on the same Wi-Fi.</Text>
        </View>
        <View style={s.introBottom}>
          {__DEV__ ? (
            // Development only (not in release builds): paste the pairing link, for emulators without a camera.
            <TextInput
              accessibilityLabel="Paste pairing link (development only)"
              placeholder="dev: paste pocketremote:// link"
              placeholderTextColor={color.textMuted}
              autoCapitalize="none"
              autoCorrect={false}
              cursorColor={color.amber}
              style={s.dev}
              onSubmitEditing={(e) => {
                handled.current = false;
                onScan(e.nativeEvent.text.trim());
              }}
            />
          ) : null}
          <Pressable accessibilityRole="button" onPress={startScan} style={s.primaryWide}>
            <Text style={s.primaryText}>Scan code</Text>
          </Pressable>
        </View>
      </View>
    );
  }

  return (
    <View style={s.fill}>
      <View style={[s.fill, s.center]}>
        {step === 'working' && (
          <>
            <Runway />
            <Text style={[t.title, s.h]}>Connecting to your computer…</Text>
          </>
        )}
        {step === 'code' && (
          <>
            <Text style={[t.body, { color: color.textMuted, marginBottom: space.m }]}>Pairing with {addr}</Text>
            <Ring>
              <Text style={s.code} accessibilityLabel={`Code ${code.split('').join(' ')}`}>
                {code.slice(0, 3)} {code.slice(3)}
              </Text>
            </Ring>
            <Text style={[t.body, s.p, { marginTop: space.l }]}>Approve on your computer. Check that the code matches the one shown there.</Text>
            <View style={s.track} accessible={false}>
              <Animated.View style={[s.trackFill, { transform: [{ scaleX: countdown }] }]} />
            </View>
          </>
        )}
        {step === 'error' && (
          <>
            <Text style={[t.title, s.h]}>Couldn&apos;t pair</Text>
            <Text style={[t.body, s.p]} accessibilityLiveRegion="polite">
              {error}
            </Text>
            <Pressable accessibilityRole="button" onPress={startScan} style={s.primary}>
              <Text style={s.primaryText}>Try again</Text>
            </Pressable>
            {permission && !permission.granted && !permission.canAskAgain ? (
              <Pressable accessibilityRole="button" onPress={() => void Linking.openSettings()} style={s.ghost}>
                <Text style={t.body}>Open Settings</Text>
              </Pressable>
            ) : null}
          </>
        )}
      </View>
      {step === 'working' || step === 'code' ? (
        <Pressable
          accessibilityRole="button"
          onPress={() => {
            cleanup();
            setStep('intro');
          }}
          style={s.cancel}
        >
          <Text style={[t.body, { color: color.textMuted }]}>Cancel</Text>
        </Pressable>
      ) : null}
    </View>
  );
}

const FRAME = 240;
const s = StyleSheet.create({
  fill: { flex: 1, backgroundColor: color.ground },
  center: { alignItems: 'center', justifyContent: 'center', padding: space.l },
  runway: { alignItems: 'center', gap: space.s, marginBottom: space.l },
  h: { textAlign: 'center', fontSize: 22, marginBottom: space.s },
  p: { textAlign: 'center', color: color.textMuted, maxWidth: 300, marginBottom: space.l, lineHeight: 22 },
  code: { fontSize: 57, fontWeight: '400', letterSpacing: 6, color: color.amber, fontVariant: ['tabular-nums'] },
  track: { width: 200, height: 2, borderRadius: 1, backgroundColor: color.surface, overflow: 'hidden', marginTop: space.s },
  trackFill: { height: 2, backgroundColor: color.amber, transformOrigin: 'left' },
  cancel: { minHeight: TARGET, alignSelf: 'center', justifyContent: 'center', paddingHorizontal: space.l, marginBottom: space.l },
  introTop: { flex: 1, alignItems: 'center', justifyContent: 'center', padding: space.l },
  introBottom: { paddingHorizontal: space.m, paddingBottom: space.l + 8, gap: space.s },
  primaryWide: { minHeight: 56, borderRadius: radius.pill, backgroundColor: color.amber, alignItems: 'center', justifyContent: 'center' },
  primary: { minHeight: 52, minWidth: 200, borderRadius: radius.pill, backgroundColor: color.amber, alignItems: 'center', justifyContent: 'center', paddingHorizontal: space.l },
  primaryText: { color: color.onAmber, fontSize: 16, fontWeight: '500' },
  dev: { height: TARGET, fontFamily: font.regular, borderRadius: radius.pill, borderWidth: 1, borderColor: color.outline, paddingHorizontal: space.m, color: color.text },
  ghost: { minHeight: TARGET, minWidth: 120, alignItems: 'center', justifyContent: 'center', marginTop: space.s },
  scrim: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(7,7,10,0.35)' },
  frame: { position: 'absolute', left: '50%', top: '38%', width: FRAME, height: FRAME, marginLeft: -FRAME / 2, marginTop: -FRAME / 2 },
  corner: { position: 'absolute', width: 28, height: 28, borderColor: color.amber },
  cTL: { left: 0, top: 0, borderLeftWidth: 4, borderTopWidth: 4, borderTopLeftRadius: 12 },
  cTR: { right: 0, top: 0, borderRightWidth: 4, borderTopWidth: 4, borderTopRightRadius: 12 },
  cBL: { left: 0, bottom: 0, borderLeftWidth: 4, borderBottomWidth: 4, borderBottomLeftRadius: 12 },
  cBR: { right: 0, bottom: 0, borderRightWidth: 4, borderBottomWidth: 4, borderBottomRightRadius: 12 },
  scanBottom: { position: 'absolute', left: 0, right: 0, bottom: 48, alignItems: 'center', gap: space.s },
});
