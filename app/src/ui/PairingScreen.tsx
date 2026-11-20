import { CameraView, useCameraPermissions } from 'expo-camera';
import { useCallback, useEffect, useRef, useState } from 'react';
import { BackHandler, Linking, Pressable, StyleSheet, Text, View } from 'react-native';
import type { Keypair } from '../noise';
import { secureRng } from '../rng';
import { PairingClient, parsePairingUri, type PairingEvent, type PairingOffer } from '../session';
import { savePairing, type Pairing } from '../storage';
import { color, radius, space, TARGET, type as t } from '../theme';
import { press, tick } from './haptics';

type Step = 'intro' | 'scan' | 'working' | 'code' | 'error';

const PAIRING_TIMEOUT_MS = 130_000; // the host's window is 120 s

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
          } else if (e.kind === 'paired') {
            const pairing: Pairing = { addr: offer.addr, hostKey: e.hostKey, hostId: offer.hostId, hostName: e.hostName };
            void savePairing(pairing).then(() => {
              cleanup();
              onPaired(pairing);
            });
          } else {
            fail("Pairing didn't complete. The Mac may have rejected it or the code expired.");
          }
        },
      });
      ws.onopen = () => client.start();
      ws.onmessage = (ev) => client.receive(new Uint8Array(ev.data as ArrayBuffer));
      ws.onclose = () => client.remoteClosed();
      ws.onerror = () => undefined;
      timer.current = setTimeout(() => fail('That took too long. Start pairing again on your Mac.'), PAIRING_TIMEOUT_MS);
    },
    [phone, onPaired, cleanup, fail],
  );

  const onScan = useCallback(
    (data: string) => {
      if (handled.current) return;
      const offer = parsePairingUri(data);
      handled.current = true;
      if (!offer) {
        fail("That isn't a PocketRemote code. Scan the code shown on your Mac.");
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
        setError(r.canAskAgain ? 'PocketRemote needs the camera to read the code on your Mac.' : 'Camera access is turned off. Enable it in Settings to scan the code.');
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
          <Text style={[t.body, { textAlign: 'center' }]}>Point at the code on your Mac</Text>
          <Pressable accessibilityRole="button" onPress={() => setStep('intro')} style={s.ghost}>
            <Text style={t.body}>Cancel</Text>
          </Pressable>
        </View>
      </View>
    );
  }

  return (
    <View style={[s.fill, s.center]}>
      {step === 'intro' && (
        <>
          <Runway />
          <Text style={[t.title, s.h]}>Scan the code on your Mac</Text>
          <Text style={[t.body, s.p]}>On your Mac, open PocketRemote and choose Pair phone. Both devices must be on the same Wi-Fi.</Text>
          <Pressable accessibilityRole="button" onPress={startScan} style={s.primary}>
            <Text style={s.primaryText}>Scan code</Text>
          </Pressable>
        </>
      )}
      {step === 'working' && (
        <>
          <Runway />
          <Text style={[t.title, s.h]}>Connecting to your Mac…</Text>
        </>
      )}
      {step === 'code' && (
        <>
          <Text style={[t.label, { marginBottom: space.m }]}>CODE</Text>
          <Text style={t.code} accessibilityLabel={`Code ${code.split('').join(' ')}`}>
            {code.slice(0, 3)} {code.slice(3)}
          </Text>
          <Text style={[t.body, s.p, { marginTop: space.l }]}>Approve on your Mac. Check that the code matches the one shown there.</Text>
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
  );
}

const FRAME = 240;
const s = StyleSheet.create({
  fill: { flex: 1, backgroundColor: color.ground },
  center: { alignItems: 'center', justifyContent: 'center', padding: space.l },
  runway: { alignItems: 'center', gap: space.s, marginBottom: space.l },
  h: { textAlign: 'center', fontSize: 22, marginBottom: space.s },
  p: { textAlign: 'center', color: color.textMuted, maxWidth: 320, marginBottom: space.l },
  primary: { minHeight: 52, minWidth: 200, borderRadius: radius.pill, backgroundColor: color.amber, alignItems: 'center', justifyContent: 'center', paddingHorizontal: space.l },
  primaryText: { color: color.onAmber, fontSize: 16, fontWeight: '500' },
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
