import { StatusBar } from 'expo-status-bar';
import { useKeepAwake } from 'expo-keep-awake';
import { useCallback, useEffect, useState } from 'react';
import { StyleSheet, useWindowDimensions, View } from 'react-native';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';
import { type Keypair } from './src/noise';
import { secureRng } from './src/rng';
import { clearPairing, loadOrCreatePhoneKeys, loadPairing, loadSettings, saveSettings, type Pairing } from './src/storage';
import { color, space } from './src/theme';
import { DEFAULT_POINTER, type PointerSettings } from './src/trackpad/pointer';
import { useRemote } from './src/useRemote';
import { KeyboardBar } from './src/ui/KeyboardBar';
import { MainPage } from './src/ui/MainPage';
import { MorePage } from './src/ui/MorePage';
import { Pager } from './src/ui/Pager';
import { PairingScreen } from './src/ui/PairingScreen';
import { SettingsSheet } from './src/ui/SettingsSheet';
import { StatusHeader } from './src/ui/StatusHeader';
import { Trackpad } from './src/ui/Trackpad';

interface Boot {
  phone: Keypair;
  pairing: Pairing | null;
  settings: PointerSettings;
}

function Remote({ boot, onForget }: { boot: Boot; onForget: () => void }) {
  useKeepAwake();
  const pairing = boot.pairing!;
  const { state, send, reconnect } = useRemote(pairing, boot.phone);
  const [settings, setSettings] = useState(boot.settings);
  const [typing, setTyping] = useState(false);
  const [sheet, setSheet] = useState(false);
  const { width, height } = useWindowDimensions();
  const landscape = width > height;
  const connected = state.phase === 'connected';
  const usable = connected && !state.status?.paused;

  const change = useCallback((s: PointerSettings) => {
    setSettings(s);
    void saveSettings(s);
  }, []);

  const header = (
    <StatusHeader
      state={state}
      hostName={pairing.hostName}
      profile={state.status?.profile ?? 'generic'}
      showProfile={(state.status?.profiles.length ?? 0) > 1}
      onReconnect={reconnect}
      onSettings={() => setSheet(true)}
      onRescan={onForget}
    />
  );

  return (
    <SafeAreaView style={s.screen} edges={['top', 'bottom', 'left', 'right']}>
      <View style={s.body}>
        {header}
        {landscape ? (
          <Trackpad send={send} settings={settings} dim={!usable} />
        ) : (
          <>
            <KeyboardBar send={send} onTyping={setTyping} />
            {!typing ? (
              <View style={[s.controls, !usable && s.dim]} pointerEvents={usable ? 'auto' : 'none'}>
                <Pager
                  labels={['Remote', 'Browser & slides']}
                  pages={[<MainPage key="main" send={send} active={usable} />, <MorePage key="more" send={send} />]}
                />
              </View>
            ) : null}
            <Trackpad send={send} settings={settings} dim={!usable} />
          </>
        )}
      </View>
      <SettingsSheet
        visible={sheet}
        settings={settings}
        hostName={pairing.hostName}
        onChange={change}
        onClose={() => setSheet(false)}
        onForget={() => {
          setSheet(false);
          onForget();
        }}
      />
    </SafeAreaView>
  );
}

export default function App() {
  const [boot, setBoot] = useState<Boot | null>(null);

  useEffect(() => {
    void (async () => {
      const [phone, pairing, settings] = await Promise.all([loadOrCreatePhoneKeys(secureRng), loadPairing(), loadSettings().catch(() => DEFAULT_POINTER)]);
      setBoot({ phone, pairing, settings });
    })();
  }, []);

  const forget = useCallback(() => {
    void clearPairing().then(() => setBoot((b) => (b ? { ...b, pairing: null } : b)));
  }, []);

  return (
    <SafeAreaProvider>
      <StatusBar style="light" />
      <View style={s.root}>
        {!boot ? null : boot.pairing ? (
          <Remote key={boot.pairing.hostId} boot={boot} onForget={forget} />
        ) : (
          <SafeAreaView style={s.screen}>
            <PairingScreen phone={boot.phone} onPaired={(p) => setBoot({ ...boot, pairing: p })} />
          </SafeAreaView>
        )}
      </View>
    </SafeAreaProvider>
  );
}

const s = StyleSheet.create({
  root: { flex: 1, backgroundColor: color.ground },
  screen: { flex: 1, backgroundColor: color.ground },
  body: { flex: 1, paddingHorizontal: space.m, paddingTop: space.s, paddingBottom: space.m, gap: space.s },
  controls: { gap: space.s },
  dim: { opacity: 0.5 },
});
