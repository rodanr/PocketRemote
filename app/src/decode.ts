// Validating decoder for app messages. Mirrors `Message::decode` in desktop/src/protocol.rs:
// exact field sets (no unknown fields), enum membership, ranges and lengths.
// Plain TypeScript: no React Native imports. Never log decoded values (typed text is private).
import {
  BROWSER_ACTION_VALUES,
  BUTTON_STATE_VALUES,
  ERROR_CODE_VALUES,
  KEY_NAME_VALUES,
  MAX_DELTA,
  MAX_HOST_NAME_CHARS,
  MAX_PLAINTEXT_BYTES,
  MAX_PROFILES,
  MAX_PROFILE_ID_CHARS,
  MAX_TEXT_SCALARS,
  MEDIA_ACTION_VALUES,
  MOUSE_BUTTON_VALUES,
  PLATFORM_VALUES,
  PLAYER_ACTION_VALUES,
  type Message,
} from './protocol';

/** Opaque failure: a bad message must close the session, with no detail. */
export class DecodeError extends Error {
  constructor() {
    super('invalid message');
    this.name = 'DecodeError';
  }
}

type Check = (v: unknown) => boolean;

const oneOf =
  (values: readonly string[]): Check =>
  (v) =>
    typeof v === 'string' && values.includes(v);
const scalars = (s: string): number => Array.from(s).length;
const delta: Check = (v) => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= MAX_DELTA;
const millis: Check = (v) => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
const bool: Check = (v) => typeof v === 'boolean';
const profileId: Check = (v) =>
  typeof v === 'string' && v.length > 0 && scalars(v) <= MAX_PROFILE_ID_CHARS && /^[a-z0-9_-]+$/.test(v);

const FIELDS: Record<Message['type'], Record<string, Check>> = {
  paired: { host_name: (v) => typeof v === 'string' && scalars(v) <= MAX_HOST_NAME_CHARS },
  status: {
    platform: oneOf(PLATFORM_VALUES),
    profile: profileId,
    profiles: (v) => Array.isArray(v) && v.length <= MAX_PROFILES && v.every(profileId),
    paused: bool,
  },
  ping: { t: millis },
  pong: { t: millis },
  move: { dx: delta, dy: delta },
  click: { button: oneOf(MOUSE_BUTTON_VALUES), count: (v) => v === 1 || v === 2 },
  button: { button: oneOf(MOUSE_BUTTON_VALUES), state: oneOf(BUTTON_STATE_VALUES) },
  scroll: { dx: delta, dy: delta },
  key: { key: oneOf(KEY_NAME_VALUES) },
  media: { action: oneOf(MEDIA_ACTION_VALUES) },
  player: { action: oneOf(PLAYER_ACTION_VALUES) },
  browser: { action: oneOf(BROWSER_ACTION_VALUES) },
  text: { value: (v) => typeof v === 'string' && scalars(v) <= MAX_TEXT_SCALARS, submit: bool },
  profile: { id: profileId },
  error: { code: oneOf(ERROR_CODE_VALUES) },
};

const HOST_ONLY = new Set<Message['type']>(['paired', 'status', 'error']);
const HOST_TO_PHONE = new Set<Message['type']>(['paired', 'status', 'error', 'ping', 'pong']);

export function decodeMessage(bytes: Uint8Array): Message {
  if (bytes.length > MAX_PLAINTEXT_BYTES) throw new DecodeError();
  let value: unknown;
  try {
    value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    throw new DecodeError();
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new DecodeError();
  const obj = value as Record<string, unknown>;
  const type = obj.type;
  if (typeof type !== 'string' || !Object.prototype.hasOwnProperty.call(FIELDS, type)) {
    throw new DecodeError();
  }
  const spec = FIELDS[type as Message['type']];
  const keys = Object.keys(obj).filter((k) => k !== 'type');
  const names = Object.keys(spec);
  if (keys.length !== names.length || !names.every((n) => Object.prototype.hasOwnProperty.call(obj, n))) {
    throw new DecodeError(); // missing or unknown field
  }
  for (const n of names) {
    if (!spec[n](obj[n])) throw new DecodeError();
  }
  return obj as unknown as Message;
}

export function encodeMessage(msg: Message): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(msg));
}

/** True for messages a phone may send to the host. */
export function isPhoneToHost(msg: Message): boolean {
  return !HOST_ONLY.has(msg.type);
}

/** True for messages the host may send to the phone. The phone must reject any other type. */
export function isHostToPhone(msg: Message): boolean {
  return HOST_TO_PHONE.has(msg.type);
}
