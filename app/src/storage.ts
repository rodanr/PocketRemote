// Secure storage (Android Keystore-backed via expo-secure-store). Security-sensitive.
// Holds the phone's static key, the pinned host key and where the host lives. Never log values.
import * as SecureStore from 'expo-secure-store';
import { fromHex, toHex } from './hex';
import { generateKeypair, type Keypair, type Rng } from './noise';
import { DEFAULT_SETTINGS, type AppSettings } from './trackpad/pointer';

const K = {
  priv: 'pr.phone.priv',
  pub: 'pr.phone.pub',
  hostKey: 'pr.host.key',
  hostId: 'pr.host.id',
  addr: 'pr.host.addr',
  hostName: 'pr.host.name',
  settings: 'pr.settings',
} as const;

export interface Pairing {
  addr: string;
  hostKey: Uint8Array;
  hostId: string;
  hostName: string;
}

export async function loadOrCreatePhoneKeys(rng: Rng): Promise<Keypair> {
  const priv = await SecureStore.getItemAsync(K.priv);
  const pub = await SecureStore.getItemAsync(K.pub);
  const p = priv ? fromHex(priv) : null;
  const q = pub ? fromHex(pub) : null;
  if (p && q && p.length === 32 && q.length === 32) return { privateKey: p, publicKey: q };
  const kp = generateKeypair(rng);
  await SecureStore.setItemAsync(K.priv, toHex(kp.privateKey));
  await SecureStore.setItemAsync(K.pub, toHex(kp.publicKey));
  return kp;
}

export async function loadPairing(): Promise<Pairing | null> {
  const [addr, key, hostId, hostName] = await Promise.all([
    SecureStore.getItemAsync(K.addr),
    SecureStore.getItemAsync(K.hostKey),
    SecureStore.getItemAsync(K.hostId),
    SecureStore.getItemAsync(K.hostName),
  ]);
  const hostKey = key ? fromHex(key) : null;
  if (!addr || !hostKey || hostKey.length !== 32 || !hostId) return null;
  return { addr, hostKey, hostId, hostName: hostName ?? 'your computer' };
}

export async function savePairing(p: Pairing): Promise<void> {
  await SecureStore.setItemAsync(K.addr, p.addr);
  await SecureStore.setItemAsync(K.hostKey, toHex(p.hostKey));
  await SecureStore.setItemAsync(K.hostId, p.hostId);
  await SecureStore.setItemAsync(K.hostName, p.hostName);
}

/** Forget the host. The phone's own key is kept so re-pairing does not change it. */
export async function clearPairing(): Promise<void> {
  await Promise.all([K.addr, K.hostKey, K.hostId, K.hostName].map((k) => SecureStore.deleteItemAsync(k)));
}

export async function loadSettings(): Promise<AppSettings> {
  try {
    const raw = await SecureStore.getItemAsync(K.settings);
    const j = raw ? (JSON.parse(raw) as Partial<AppSettings>) : {};
    const clamp = (v: unknown, lo: number, hi: number, d: number) =>
      typeof v === 'number' && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : d;
    return {
      speed: clamp(j.speed, 0.5, 4, DEFAULT_SETTINGS.speed),
      acceleration: clamp(j.acceleration, 0, 1, DEFAULT_SETTINGS.acceleration),
      naturalScroll: typeof j.naturalScroll === 'boolean' ? j.naturalScroll : DEFAULT_SETTINGS.naturalScroll,
      scrollSpeed: clamp(j.scrollSpeed, 0.5, 3, DEFAULT_SETTINGS.scrollSpeed),
      haptics: typeof j.haptics === 'boolean' ? j.haptics : DEFAULT_SETTINGS.haptics,
      keepAwake: typeof j.keepAwake === 'boolean' ? j.keepAwake : DEFAULT_SETTINGS.keepAwake,
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export async function saveSettings(s: AppSettings): Promise<void> {
  await SecureStore.setItemAsync(K.settings, JSON.stringify(s));
}
