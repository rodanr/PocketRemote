//! App messages (PLAN.md §4.2). This file is the source of truth: `app/src/protocol.ts` is
//! generated from it. Regenerate with `UPDATE_PROTOCOL_TS=1 cargo test protocol`.
//!
//! Never add a command that runs a shell, touches files, opens URLs, reads the clipboard or
//! sends arbitrary key combinations (PLAN.md §3.9).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Largest decrypted application message, in bytes (a Noise transport message is ≤ 4096).
pub const MAX_PLAINTEXT_BYTES: usize = 4096;
/// Largest `text.value`, in Unicode scalar values.
pub const MAX_TEXT_SCALARS: usize = 500;
/// Largest absolute `dx` / `dy` accepted in one `move` or `scroll`, in pixels.
pub const MAX_DELTA: f32 = 10_000.0;
pub const MAX_HOST_NAME_CHARS: usize = 64;
pub const MAX_PROFILE_ID_CHARS: usize = 32;
pub const MAX_PROFILES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ButtonState {
    Down,
    Up,
}

/// Named keys only. There is deliberately no way to send a modifier or an arbitrary key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum KeyName {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    PlayPause,
    Next,
    Prev,
    Stop,
    Mute,
    VolUp,
    VolDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlayerAction {
    SeekBack,
    SeekFwd,
    Fullscreen,
    Captions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    Back,
    Forward,
    Reload,
    PrevTab,
    NextTab,
    CloseTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Paused,
    Revoked,
    Replaced,
    RateLimited,
    Idle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Message {
    /// host → phone, only at the end of a pairing session.
    Paired {
        host_name: String,
    },
    /// host → phone, after the handshake and on change.
    Status {
        platform: Platform,
        profile: String,
        profiles: Vec<String>,
        paused: bool,
    },
    Ping {
        #[cfg_attr(test, ts(type = "number"))]
        t: u64,
    },
    Pong {
        #[cfg_attr(test, ts(type = "number"))]
        t: u64,
    },
    Move {
        dx: f32,
        dy: f32,
    },
    Click {
        button: MouseButton,
        count: u8,
    },
    Button {
        button: MouseButton,
        state: ButtonState,
    },
    Scroll {
        dx: f32,
        dy: f32,
    },
    Key {
        key: KeyName,
    },
    Media {
        action: MediaAction,
    },
    Player {
        action: PlayerAction,
    },
    Browser {
        action: BrowserAction,
    },
    Text {
        value: String,
        submit: bool,
    },
    Profile {
        id: String,
    },
    /// host → phone, then the host closes the session.
    Error {
        code: ErrorCode,
    },
}

/// Why a frame was rejected. Carries no payload text, so it is safe to log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    TooLarge,
    Malformed,
    Invalid(&'static str),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => f.write_str("message too large"),
            Self::Malformed => f.write_str("malformed message"),
            Self::Invalid(why) => write!(f, "invalid message: {why}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl Message {
    /// Decode and validate one plaintext app message. Any failure must close the session.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() > MAX_PLAINTEXT_BYTES {
            return Err(DecodeError::TooLarge);
        }
        let msg: Self = serde_json::from_slice(bytes).map_err(|_| DecodeError::Malformed)?;
        msg.validate()?;
        Ok(msg)
    }

    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Message always serializes")
    }

    /// Range and length checks that serde cannot express.
    pub fn validate(&self) -> Result<(), DecodeError> {
        let delta_ok = |v: f32| v.is_finite() && v.abs() <= MAX_DELTA;
        match self {
            Self::Move { dx, dy } | Self::Scroll { dx, dy } => {
                if !(delta_ok(*dx) && delta_ok(*dy)) {
                    return Err(DecodeError::Invalid("delta out of range"));
                }
            }
            Self::Click { count, .. } => {
                if !(1..=2).contains(count) {
                    return Err(DecodeError::Invalid("click count must be 1 or 2"));
                }
            }
            Self::Text { value, .. } => {
                if value.chars().count() > MAX_TEXT_SCALARS {
                    return Err(DecodeError::Invalid("text too long"));
                }
            }
            Self::Paired { host_name } => {
                if host_name.chars().count() > MAX_HOST_NAME_CHARS {
                    return Err(DecodeError::Invalid("host name too long"));
                }
            }
            Self::Profile { id } => check_profile_id(id)?,
            Self::Status {
                profile, profiles, ..
            } => {
                check_profile_id(profile)?;
                if profiles.len() > MAX_PROFILES {
                    return Err(DecodeError::Invalid("too many profiles"));
                }
                profiles.iter().try_for_each(|p| check_profile_id(p))?;
            }
            Self::Ping { .. }
            | Self::Pong { .. }
            | Self::Button { .. }
            | Self::Key { .. }
            | Self::Media { .. }
            | Self::Player { .. }
            | Self::Browser { .. }
            | Self::Error { .. } => {}
        }
        Ok(())
    }

    /// True for messages a phone may send to the host. The host must reject any other type.
    pub fn is_phone_to_host(&self) -> bool {
        !matches!(
            self,
            Self::Paired { .. } | Self::Status { .. } | Self::Error { .. }
        )
    }

    /// True for messages the host may send to the phone. The phone must reject any other type.
    pub fn is_host_to_phone(&self) -> bool {
        matches!(
            self,
            Self::Paired { .. }
                | Self::Status { .. }
                | Self::Error { .. }
                | Self::Ping { .. }
                | Self::Pong { .. }
        )
    }
}

fn check_profile_id(id: &str) -> Result<(), DecodeError> {
    let ok = !id.is_empty()
        && id.chars().count() <= MAX_PROFILE_ID_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::Invalid("bad profile id"))
    }
}

/// Remove control characters (U+0000–U+001F, U+007F–U+009F) before typing (PLAN.md §3.9).
pub fn sanitize_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(*c as u32, 0x00..=0x1F | 0x7F..=0x9F))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use ts_rs::{Config, TS};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn fixtures(kind: &str) -> Vec<(String, Vec<u8>)> {
        let dir = repo_root().join("protocol/fixtures").join(kind);
        let mut out: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&p).unwrap(),
                )
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn valid_fixtures_decode_and_round_trip() {
        let all = fixtures("valid");
        assert!(
            all.len() >= 15,
            "need at least one fixture per message type"
        );
        for (name, bytes) in all {
            let msg = Message::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            let again: serde_json::Value = serde_json::from_slice(&msg.encode()).unwrap();
            let orig: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(again, orig, "{name}: round trip changed the message");
        }
    }

    #[test]
    fn every_message_type_has_a_valid_fixture() {
        let mut seen = std::collections::BTreeSet::new();
        for (_, bytes) in fixtures("valid") {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            seen.insert(v["type"].as_str().unwrap().to_owned());
        }
        for t in [
            "paired", "status", "ping", "pong", "move", "click", "button", "scroll", "key",
            "media", "player", "browser", "text", "profile", "error",
        ] {
            assert!(seen.contains(t), "no valid fixture for type {t}");
        }
    }

    #[test]
    fn invalid_fixtures_are_rejected() {
        let all = fixtures("invalid");
        assert!(all.len() >= 12);
        for (name, bytes) in all {
            assert!(Message::decode(&bytes).is_err(), "{name} was accepted");
        }
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let big = vec![b' '; MAX_PLAINTEXT_BYTES + 1];
        assert_eq!(Message::decode(&big), Err(DecodeError::TooLarge));
    }

    #[test]
    fn text_limit_counts_scalar_values_not_bytes() {
        let ok = serde_json::json!({"type":"text","value":"é".repeat(500),"submit":false});
        assert!(Message::decode(ok.to_string().as_bytes()).is_ok());
        let bad = serde_json::json!({"type":"text","value":"é".repeat(501),"submit":false});
        assert!(Message::decode(bad.to_string().as_bytes()).is_err());
    }

    #[test]
    fn sanitize_strips_control_characters_only() {
        assert_eq!(
            sanitize_text("a\u{0}b\nc\td\u{7f}e\u{85}f\u{9f}g"),
            "abcdefg"
        );
        assert_eq!(sanitize_text("héllo wörld ✓"), "héllo wörld ✓");
    }

    #[test]
    fn directions_are_disjoint_where_required() {
        let key = Message::Key { key: KeyName::Up };
        assert!(key.is_phone_to_host() && !key.is_host_to_phone());
        let st = Message::Error {
            code: ErrorCode::Idle,
        };
        assert!(!st.is_phone_to_host() && st.is_host_to_phone());
        let ping = Message::Ping { t: 1 };
        assert!(ping.is_phone_to_host() && ping.is_host_to_phone());
    }

    fn generate_typescript() -> String {
        let cfg = Config::new();
        let decls = [
            Platform::decl(&cfg),
            MouseButton::decl(&cfg),
            ButtonState::decl(&cfg),
            KeyName::decl(&cfg),
            MediaAction::decl(&cfg),
            PlayerAction::decl(&cfg),
            BrowserAction::decl(&cfg),
            ErrorCode::decl(&cfg),
            Message::decl(&cfg),
        ];
        let mut out = String::from(
            "// GENERATED from desktop/src/protocol.rs. DO NOT EDIT BY HAND.\n\
             // Regenerate: UPDATE_PROTOCOL_TS=1 cargo test protocol   (in desktop/)\n\n",
        );
        out.push_str(&format!(
            "export const MAX_PLAINTEXT_BYTES = {MAX_PLAINTEXT_BYTES};\n"
        ));
        out.push_str(&format!(
            "export const MAX_TEXT_SCALARS = {MAX_TEXT_SCALARS};\n"
        ));
        out.push_str(&format!("export const MAX_DELTA = {MAX_DELTA};\n"));
        out.push_str(&format!(
            "export const MAX_HOST_NAME_CHARS = {MAX_HOST_NAME_CHARS};\n"
        ));
        out.push_str(&format!(
            "export const MAX_PROFILE_ID_CHARS = {MAX_PROFILE_ID_CHARS};\n"
        ));
        out.push_str(&format!("export const MAX_PROFILES = {MAX_PROFILES};\n\n"));
        for d in decls {
            out.push_str("export ");
            out.push_str(&d);
            out.push_str("\n\n");
        }
        out.trim_end().to_owned() + "\n"
    }

    #[test]
    fn generated_typescript_is_up_to_date() {
        let path = repo_root().join("app/src/protocol.ts");
        let want = generate_typescript();
        if std::env::var_os("UPDATE_PROTOCOL_TS").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &want).unwrap();
            return;
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            have, want,
            "app/src/protocol.ts is stale: run UPDATE_PROTOCOL_TS=1 cargo test protocol"
        );
    }
}
