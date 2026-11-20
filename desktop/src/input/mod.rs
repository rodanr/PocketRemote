//! Input injection (PLAN.md §5.2). The `InputBackend` trait is the only thing the executor
//! touches; real backends (enigo on macOS and Windows) and a recording mock implement it.
//!
//! Only the actions in PLAN.md §4.2 exist. `Chord` is internal: it lets the executor express a
//! fixed shortcut such as "Cmd+R for reload". The phone can never send one.

pub mod mock;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod enigo_backend;

use crate::protocol::{ButtonState, KeyName, MediaAction, MouseButton};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// The OS refused input injection (macOS: Accessibility permission missing).
    NoPermission,
    /// This backend cannot do that action on this OS.
    Unsupported,
    Failed,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoPermission => "input permission missing",
            Self::Unsupported => "action not supported on this system",
            Self::Failed => "input injection failed",
        })
    }
}

impl std::error::Error for InputError {}

pub type Result<T> = std::result::Result<T, InputError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Command on macOS, Ctrl elsewhere is chosen by the executor, not here.
    Cmd,
    Ctrl,
    Alt,
    Shift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordKey {
    Named(KeyName),
    Char(char),
}

/// A fixed shortcut chosen by the executor's own tables. Not reachable from the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: Vec<Modifier>,
    pub key: ChordKey,
}

pub trait InputBackend: Send {
    /// Relative pointer move in whole pixels.
    fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()>;
    fn button(&mut self, button: MouseButton, state: ButtonState) -> Result<()>;
    /// `count` is 1 or 2.
    fn click(&mut self, button: MouseButton, count: u8) -> Result<()>;
    /// Wheel notches; positive is down / right.
    fn scroll(&mut self, dx: i32, dy: i32) -> Result<()>;
    /// Press and release one named key.
    fn key(&mut self, key: KeyName) -> Result<()>;
    /// Press modifiers, click the key, release modifiers (always, even on error).
    fn chord(&mut self, chord: &Chord) -> Result<()>;
    /// Type already-sanitised text.
    fn text(&mut self, text: &str) -> Result<()>;
    fn media(&mut self, action: MediaAction) -> Result<()>;
}
