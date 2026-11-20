// GENERATED from desktop/src/protocol.rs. DO NOT EDIT BY HAND.
// Regenerate: UPDATE_PROTOCOL_TS=1 cargo test protocol   (in desktop/)

export const MAX_PLAINTEXT_BYTES = 4096;
export const MAX_TEXT_SCALARS = 500;
export const MAX_DELTA = 10000;
export const MAX_HOST_NAME_CHARS = 64;
export const MAX_PROFILE_ID_CHARS = 32;
export const MAX_PROFILES = 16;

export type Platform = "macos" | "windows" | "linux";

export const PLATFORM_VALUES = ["macos", "windows", "linux"] as const;

export type MouseButton = "left" | "right" | "middle";

export const MOUSE_BUTTON_VALUES = ["left", "right", "middle"] as const;

export type ButtonState = "down" | "up";

export const BUTTON_STATE_VALUES = ["down", "up"] as const;

export type KeyName = "up" | "down" | "left" | "right" | "enter" | "escape" | "backspace" | "tab" | "space" | "home" | "end" | "page_up" | "page_down";

export const KEY_NAME_VALUES = ["up", "down", "left", "right", "enter", "escape", "backspace", "tab", "space", "home", "end", "page_up", "page_down"] as const;

export type MediaAction = "play_pause" | "next" | "prev" | "stop" | "mute" | "vol_up" | "vol_down";

export const MEDIA_ACTION_VALUES = ["play_pause", "next", "prev", "stop", "mute", "vol_up", "vol_down"] as const;

export type PlayerAction = "seek_back" | "seek_fwd" | "fullscreen" | "captions";

export const PLAYER_ACTION_VALUES = ["seek_back", "seek_fwd", "fullscreen", "captions"] as const;

export type BrowserAction = "back" | "forward" | "reload" | "prev_tab" | "next_tab" | "close_tab";

export const BROWSER_ACTION_VALUES = ["back", "forward", "reload", "prev_tab", "next_tab", "close_tab"] as const;

export type ErrorCode = "paused" | "revoked" | "replaced" | "rate_limited" | "idle";

export const ERROR_CODE_VALUES = ["paused", "revoked", "replaced", "rate_limited", "idle"] as const;

export type Message = { "type": "paired", host_name: string, } | { "type": "status", platform: Platform, profile: string, profiles: Array<string>, paused: boolean, } | { "type": "ping", t: number, } | { "type": "pong", t: number, } | { "type": "move", dx: number, dy: number, } | { "type": "click", button: MouseButton, count: number, } | { "type": "button", button: MouseButton, state: ButtonState, } | { "type": "scroll", dx: number, dy: number, } | { "type": "key", key: KeyName, } | { "type": "media", action: MediaAction, } | { "type": "player", action: PlayerAction, } | { "type": "browser", action: BrowserAction, } | { "type": "text", value: string, submit: boolean, } | { "type": "profile", id: string, } | { "type": "error", code: ErrorCode, };
