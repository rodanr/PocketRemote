//! Recording backend for tests: every call is appended to a shared log.

use super::{Chord, InputBackend, Result};
use crate::protocol::{ButtonState, KeyName, MediaAction, MouseButton};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    MoveRel(i32, i32),
    Button(MouseButton, ButtonState),
    Click(MouseButton, u8),
    Scroll(i32, i32),
    Key(KeyName),
    Chord(Chord),
    Text(String),
    Media(MediaAction),
}

#[derive(Clone, Default)]
pub struct MockBackend {
    log: Arc<Mutex<Vec<Call>>>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle to the log that stays valid after the backend moves to another thread.
    pub fn log(&self) -> Arc<Mutex<Vec<Call>>> {
        self.log.clone()
    }

    fn push(&self, c: Call) -> Result<()> {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).push(c);
        Ok(())
    }
}

impl InputBackend for MockBackend {
    fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()> {
        self.push(Call::MoveRel(dx, dy))
    }
    fn button(&mut self, b: MouseButton, s: ButtonState) -> Result<()> {
        self.push(Call::Button(b, s))
    }
    fn click(&mut self, b: MouseButton, n: u8) -> Result<()> {
        self.push(Call::Click(b, n))
    }
    fn scroll(&mut self, dx: i32, dy: i32) -> Result<()> {
        self.push(Call::Scroll(dx, dy))
    }
    fn key(&mut self, k: KeyName) -> Result<()> {
        self.push(Call::Key(k))
    }
    fn chord(&mut self, c: &Chord) -> Result<()> {
        self.push(Call::Chord(c.clone()))
    }
    fn text(&mut self, t: &str) -> Result<()> {
        self.push(Call::Text(t.to_owned()))
    }
    fn media(&mut self, a: MediaAction) -> Result<()> {
        self.push(Call::Media(a))
    }
}
