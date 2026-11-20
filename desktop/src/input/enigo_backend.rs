//! Real input on macOS and Windows via `enigo`.
//!
//! macOS needs the Accessibility permission: `new` returns `NoPermission` without it.
//! Windows cannot inject into elevated windows (UIPI). `stop` has no enigo key, so it is
//! reported as unsupported rather than faked. Scrolling is in wheel notches for now.

use super::{Chord, ChordKey, InputBackend, InputError, Modifier, Result};
use crate::protocol::{ButtonState, KeyName, MediaAction, MouseButton};
use enigo::{
    Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, NewConError, Settings,
};

pub struct EnigoBackend {
    enigo: Enigo,
}

impl EnigoBackend {
    pub fn new() -> Result<Self> {
        let settings = Settings {
            // Release anything still held if this backend is dropped (PLAN.md §3.5.7).
            release_keys_when_dropped: true,
            ..Settings::default()
        };
        match Enigo::new(&settings) {
            Ok(enigo) => Ok(Self { enigo }),
            Err(NewConError::NoPermission) => Err(InputError::NoPermission),
            Err(_) => Err(InputError::Failed),
        }
    }
}

fn btn(b: MouseButton) -> Button {
    match b {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
    }
}

fn key(k: KeyName) -> Key {
    match k {
        KeyName::Up => Key::UpArrow,
        KeyName::Down => Key::DownArrow,
        KeyName::Left => Key::LeftArrow,
        KeyName::Right => Key::RightArrow,
        KeyName::Enter => Key::Return,
        KeyName::Escape => Key::Escape,
        KeyName::Backspace => Key::Backspace,
        KeyName::Tab => Key::Tab,
        KeyName::Space => Key::Space,
        KeyName::Home => Key::Home,
        KeyName::End => Key::End,
        KeyName::PageUp => Key::PageUp,
        KeyName::PageDown => Key::PageDown,
    }
}

fn modifier(m: Modifier) -> Key {
    match m {
        Modifier::Cmd => Key::Meta,
        Modifier::Ctrl => Key::Control,
        Modifier::Alt => Key::Alt,
        Modifier::Shift => Key::Shift,
    }
}

fn io<T>(r: enigo::InputResult<T>) -> Result<T> {
    r.map_err(|_| InputError::Failed)
}

impl InputBackend for EnigoBackend {
    fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()> {
        io(self.enigo.move_mouse(dx, dy, Coordinate::Rel))
    }

    fn button(&mut self, b: MouseButton, s: ButtonState) -> Result<()> {
        let d = match s {
            ButtonState::Down => Direction::Press,
            ButtonState::Up => Direction::Release,
        };
        io(self.enigo.button(btn(b), d))
    }

    fn click(&mut self, b: MouseButton, count: u8) -> Result<()> {
        for _ in 0..count.clamp(1, 2) {
            io(self.enigo.button(btn(b), Direction::Click))?;
        }
        Ok(())
    }

    fn scroll(&mut self, dx: i32, dy: i32) -> Result<()> {
        if dy != 0 {
            io(self.enigo.scroll(dy, Axis::Vertical))?;
        }
        if dx != 0 {
            io(self.enigo.scroll(dx, Axis::Horizontal))?;
        }
        Ok(())
    }

    fn key(&mut self, k: KeyName) -> Result<()> {
        io(self.enigo.key(key(k), Direction::Click))
    }

    fn chord(&mut self, c: &Chord) -> Result<()> {
        let target = match c.key {
            ChordKey::Named(k) => key(k),
            ChordKey::Char(ch) => Key::Unicode(ch),
        };
        let mut pressed = Vec::new();
        let mut result = Ok(());
        for m in &c.modifiers {
            match io(self.enigo.key(modifier(*m), Direction::Press)) {
                Ok(()) => pressed.push(*m),
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        if result.is_ok() {
            result = io(self.enigo.key(target, Direction::Click));
        }
        // Always release what we pressed, last first, even after an error.
        for m in pressed.into_iter().rev() {
            let _ = self.enigo.key(modifier(m), Direction::Release);
        }
        result
    }

    fn text(&mut self, t: &str) -> Result<()> {
        io(self.enigo.text(t))
    }

    fn media(&mut self, a: MediaAction) -> Result<()> {
        let k = match a {
            MediaAction::PlayPause => Key::MediaPlayPause,
            MediaAction::Next => Key::MediaNextTrack,
            MediaAction::Prev => Key::MediaPrevTrack,
            MediaAction::Mute => Key::VolumeMute,
            MediaAction::VolUp => Key::VolumeUp,
            MediaAction::VolDown => Key::VolumeDown,
            MediaAction::Stop => return Err(InputError::Unsupported),
        };
        io(self.enigo.key(k, Direction::Click))
    }
}
