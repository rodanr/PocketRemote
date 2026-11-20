//! Executor (PLAN.md §5.1, §5.3): turns validated phone messages into backend calls on a
//! dedicated input thread, behind a bounded queue (capacity 64).
//!
//! * Consecutive `move` messages that are queued together are merged into one; the fractional
//!   part is carried over so slow drags never lose sub-pixel motion. Same for `scroll`.
//! * Held mouse buttons are tracked and released when a session ends.
//! * The phone can only name actions from the protocol; the key chords below are fixed tables.
//!
//! Never log message contents (typed text is private).

use crate::input::{Chord, ChordKey, InputBackend, Modifier};
use crate::protocol::{
    BrowserAction, ButtonState, KeyName, Message, MouseButton, PlayerAction, sanitize_text,
};
use crate::server::CommandSink;
use std::collections::BTreeSet;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel, sync_channel};
use std::thread::{self, JoinHandle};

pub const QUEUE_CAPACITY: usize = 64;
/// Pixels of phone scrolling per wheel notch.
pub const PIXELS_PER_NOTCH: f32 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Macos,
    Windows,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

fn chord(modifiers: &[Modifier], key: ChordKey) -> Chord {
    Chord {
        modifiers: modifiers.to_vec(),
        key,
    }
}

/// Browser shortcuts. macOS uses Cmd, elsewhere Ctrl (and Alt+Arrow for history), following the
/// browsers' own shortcut tables. These are to be re-verified against Chrome and Safari docs
/// during the on-device test (PLAN.md §5.3).
pub fn browser_chord(os: Os, action: BrowserAction) -> Chord {
    use ChordKey::{Char, Named};
    use Modifier::{Alt, Cmd, Ctrl, Shift};
    let mac = os == Os::Macos;
    match action {
        BrowserAction::Back if mac => chord(&[Cmd], Char('[')),
        BrowserAction::Forward if mac => chord(&[Cmd], Char(']')),
        BrowserAction::Back => chord(&[Alt], Named(KeyName::Left)),
        BrowserAction::Forward => chord(&[Alt], Named(KeyName::Right)),
        BrowserAction::Reload if mac => chord(&[Cmd], Char('r')),
        BrowserAction::Reload => chord(&[Ctrl], Char('r')),
        BrowserAction::PrevTab => chord(&[Ctrl, Shift], Named(KeyName::Tab)),
        BrowserAction::NextTab => chord(&[Ctrl], Named(KeyName::Tab)),
        BrowserAction::CloseTab if mac => chord(&[Cmd], Char('w')),
        BrowserAction::CloseTab => chord(&[Ctrl], Char('w')),
    }
}

/// A player profile maps the four `player` actions to fixed keys.
#[derive(Debug, Clone)]
pub struct Profile {
    pub id: &'static str,
    pub seek_back: Chord,
    pub seek_fwd: Chord,
    pub fullscreen: Chord,
    pub captions: Chord,
}

impl Profile {
    /// Works in most players: arrow keys seek, `f` fullscreen, `c` captions.
    pub fn generic() -> Self {
        Self {
            id: "generic",
            seek_back: chord(&[], ChordKey::Named(KeyName::Left)),
            seek_fwd: chord(&[], ChordKey::Named(KeyName::Right)),
            fullscreen: chord(&[], ChordKey::Char('f')),
            captions: chord(&[], ChordKey::Char('c')),
        }
    }

    pub fn chord_for(&self, action: PlayerAction) -> &Chord {
        match action {
            PlayerAction::SeekBack => &self.seek_back,
            PlayerAction::SeekFwd => &self.seek_fwd,
            PlayerAction::Fullscreen => &self.fullscreen,
            PlayerAction::Captions => &self.captions,
        }
    }
}

enum Command {
    Msg(Message),
    ReleaseAll,
    Barrier(Sender<()>),
    Shutdown,
}

/// Merge neighbouring `move` and neighbouring `scroll` commands (order of everything else kept).
fn coalesce(batch: Vec<Command>) -> Vec<Command> {
    let mut out: Vec<Command> = Vec::with_capacity(batch.len());
    for c in batch {
        match (&c, out.last_mut()) {
            (
                Command::Msg(Message::Move { dx, dy }),
                Some(Command::Msg(Message::Move { dx: ax, dy: ay })),
            ) => {
                *ax += dx;
                *ay += dy;
            }
            (
                Command::Msg(Message::Scroll { dx, dy }),
                Some(Command::Msg(Message::Scroll { dx: ax, dy: ay })),
            ) => {
                *ax += dx;
                *ay += dy;
            }
            _ => out.push(c),
        }
    }
    out
}

/// Split an accumulated value into whole units and a remainder that is carried over.
fn take_whole(acc: &mut f32) -> i32 {
    let whole = acc.trunc();
    *acc -= whole;
    whole as i32
}

struct Worker {
    backend: Box<dyn InputBackend>,
    os: Os,
    profile: Profile,
    held: BTreeSet<u8>,
    move_rem: (f32, f32),
    scroll_rem: (f32, f32),
}

fn button_id(b: MouseButton) -> u8 {
    match b {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
}

fn button_from_id(id: u8) -> MouseButton {
    match id {
        0 => MouseButton::Left,
        1 => MouseButton::Right,
        _ => MouseButton::Middle,
    }
}

impl Worker {
    fn handle(&mut self, msg: Message) {
        // Input is best effort: a failed injection must not take the session down.
        let _ = match msg {
            Message::Move { dx, dy } => {
                self.move_rem.0 += dx;
                self.move_rem.1 += dy;
                let (x, y) = (
                    take_whole(&mut self.move_rem.0),
                    take_whole(&mut self.move_rem.1),
                );
                if x != 0 || y != 0 {
                    self.backend.move_rel(x, y)
                } else {
                    Ok(())
                }
            }
            Message::Click { button, count } => self.backend.click(button, count),
            Message::Button { button, state } => {
                match state {
                    ButtonState::Down => self.held.insert(button_id(button)),
                    ButtonState::Up => self.held.remove(&button_id(button)),
                };
                self.backend.button(button, state)
            }
            Message::Scroll { dx, dy } => {
                self.scroll_rem.0 += dx / PIXELS_PER_NOTCH;
                self.scroll_rem.1 += dy / PIXELS_PER_NOTCH;
                let (x, y) = (
                    take_whole(&mut self.scroll_rem.0),
                    take_whole(&mut self.scroll_rem.1),
                );
                if x != 0 || y != 0 {
                    self.backend.scroll(x, y)
                } else {
                    Ok(())
                }
            }
            Message::Key { key } => self.backend.key(key),
            Message::Media { action } => self.backend.media(action),
            Message::Player { action } => {
                let c = self.profile.chord_for(action).clone();
                self.backend.chord(&c)
            }
            Message::Browser { action } => self.backend.chord(&browser_chord(self.os, action)),
            Message::Text { value, submit } => {
                let clean = sanitize_text(&value);
                let typed = if clean.is_empty() {
                    Ok(())
                } else {
                    self.backend.text(&clean)
                };
                if submit {
                    typed.and(self.backend.key(KeyName::Enter))
                } else {
                    typed
                }
            }
            // Only the generic profile exists in v0.1; unknown ids are ignored.
            Message::Profile { id } => {
                if id == "generic" {
                    self.profile = Profile::generic();
                }
                Ok(())
            }
            // Host-to-phone and keepalive messages never reach the executor.
            Message::Paired { .. }
            | Message::Status { .. }
            | Message::Ping { .. }
            | Message::Pong { .. }
            | Message::Error { .. } => Ok(()),
        };
    }

    fn release_all(&mut self) {
        for id in std::mem::take(&mut self.held) {
            let _ = self.backend.button(button_from_id(id), ButtonState::Up);
        }
        self.move_rem = (0.0, 0.0);
        self.scroll_rem = (0.0, 0.0);
    }

    fn run(mut self, rx: Receiver<Command>) {
        while let Ok(first) = rx.recv() {
            let mut batch = vec![first];
            while let Ok(c) = rx.try_recv() {
                batch.push(c);
            }
            for c in coalesce(batch) {
                match c {
                    Command::Msg(m) => self.handle(m),
                    Command::ReleaseAll => self.release_all(),
                    Command::Barrier(done) => {
                        let _ = done.send(());
                    }
                    Command::Shutdown => {
                        self.release_all();
                        return;
                    }
                }
            }
        }
        self.release_all();
    }
}

/// Handle to the input thread. Implements [`CommandSink`] for the server.
pub struct Executor {
    tx: SyncSender<Command>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Executor {
    pub fn spawn(backend: Box<dyn InputBackend>, os: Os) -> Self {
        let (tx, rx) = sync_channel(QUEUE_CAPACITY);
        let worker = Worker {
            backend,
            os,
            profile: Profile::generic(),
            held: BTreeSet::new(),
            move_rem: (0.0, 0.0),
            scroll_rem: (0.0, 0.0),
        };
        let thread = thread::Builder::new()
            .name("pocketremote-input".into())
            .spawn(move || worker.run(rx))
            .expect("spawn input thread");
        Self {
            tx,
            thread: Mutex::new(Some(thread)),
        }
    }

    /// Wait until everything queued so far has been processed. For tests and shutdown.
    pub fn flush(&self) {
        let (tx, rx) = channel();
        if self.tx.send(Command::Barrier(tx)).is_ok() {
            let _ = rx.recv();
        }
    }
}

impl CommandSink for Executor {
    fn handle(&self, msg: &Message) {
        // Never block the network task: if the queue is full the command is dropped. The
        // phone sends moves continuously, so a dropped move is invisible; the 250 msg/s cap
        // and the merging make a full queue a sign of a stuck backend.
        let _ = self.tx.try_send(Command::Msg(msg.clone()));
    }

    fn release_all(&self) {
        // Must not be lost: a blocking send waits for room (the worker drains quickly).
        let _ = self.tx.send(Command::ReleaseAll);
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(t) = self.thread.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::mock::{Call, MockBackend};
    use crate::protocol::MediaAction;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn run(os: Os, msgs: &[Message]) -> Vec<Call> {
        let mock = MockBackend::new();
        let log = mock.log();
        let ex = Executor::spawn(Box::new(mock), os);
        for m in msgs {
            ex.handle(m);
        }
        ex.flush();
        log.lock().unwrap().clone()
    }

    fn mv(dx: f32, dy: f32) -> Message {
        Message::Move { dx, dy }
    }

    #[test]
    fn click_button_and_key_commands_map_one_to_one() {
        let calls = run(
            Os::Macos,
            &[
                Message::Click {
                    button: MouseButton::Left,
                    count: 2,
                },
                Message::Click {
                    button: MouseButton::Right,
                    count: 1,
                },
                Message::Button {
                    button: MouseButton::Left,
                    state: ButtonState::Down,
                },
                Message::Button {
                    button: MouseButton::Left,
                    state: ButtonState::Up,
                },
                Message::Key {
                    key: KeyName::PageDown,
                },
            ],
        );
        assert_eq!(
            calls,
            vec![
                Call::Click(MouseButton::Left, 2),
                Call::Click(MouseButton::Right, 1),
                Call::Button(MouseButton::Left, ButtonState::Down),
                Call::Button(MouseButton::Left, ButtonState::Up),
                Call::Key(KeyName::PageDown),
            ]
        );
    }

    #[test]
    fn every_media_action_is_forwarded() {
        let actions = [
            MediaAction::PlayPause,
            MediaAction::Next,
            MediaAction::Prev,
            MediaAction::Stop,
            MediaAction::Mute,
            MediaAction::VolUp,
            MediaAction::VolDown,
        ];
        let msgs: Vec<_> = actions
            .iter()
            .map(|&action| Message::Media { action })
            .collect();
        let calls = run(Os::Macos, &msgs);
        let want: Vec<_> = actions.iter().map(|&a| Call::Media(a)).collect();
        assert_eq!(calls, want);
    }

    #[test]
    fn fractional_motion_is_never_lost() {
        // Five 0.4 px moves are 2 px in total, delivered as whole pixels.
        let calls = run(Os::Macos, &vec![mv(0.4, 0.0); 5]);
        let (mut x, mut y) = (0, 0);
        for c in &calls {
            if let Call::MoveRel(dx, dy) = c {
                x += dx;
                y += dy;
            }
        }
        assert_eq!((x, y), (2, 0));
        assert!(calls.iter().all(|c| matches!(c, Call::MoveRel(..))));
    }

    #[test]
    fn negative_fractions_carry_over_too() {
        let calls = run(Os::Macos, &[mv(-0.6, 0.0), mv(-0.6, 0.0)]);
        let total: i32 = calls
            .iter()
            .map(|c| if let Call::MoveRel(dx, _) = c { *dx } else { 0 })
            .sum();
        assert_eq!(total, -1);
    }

    #[test]
    fn sub_pixel_moves_issue_no_call() {
        assert!(run(Os::Macos, &[mv(0.3, 0.3)]).is_empty());
    }

    #[test]
    fn queued_moves_are_merged_but_order_with_clicks_is_kept() {
        let batch = vec![
            Command::Msg(mv(1.0, 1.0)),
            Command::Msg(mv(2.0, 0.0)),
            Command::Msg(Message::Click {
                button: MouseButton::Left,
                count: 1,
            }),
            Command::Msg(mv(5.0, 5.0)),
            Command::Msg(mv(1.0, 1.0)),
        ];
        let out = coalesce(batch);
        assert_eq!(out.len(), 3);
        assert!(
            matches!(&out[0], Command::Msg(Message::Move { dx, dy }) if *dx == 3.0 && *dy == 1.0)
        );
        assert!(matches!(&out[1], Command::Msg(Message::Click { .. })));
        assert!(
            matches!(&out[2], Command::Msg(Message::Move { dx, dy }) if *dx == 6.0 && *dy == 6.0)
        );
    }

    #[test]
    fn scroll_pixels_become_notches_with_carry() {
        // 40 px per notch: 14.5 px three times is 43.5 px, so one notch and 3.5 px left over.
        let calls = run(Os::Macos, &vec![Message::Scroll { dx: 0.0, dy: 14.5 }; 3]);
        let total: i32 = calls
            .iter()
            .map(|c| if let Call::Scroll(_, y) = c { *y } else { 0 })
            .sum();
        assert_eq!(total, 1);
        let calls = run(Os::Macos, &[Message::Scroll { dx: 0.0, dy: -80.0 }]);
        assert_eq!(calls, vec![Call::Scroll(0, -2)]);
    }

    #[test]
    fn text_is_sanitised_and_submit_presses_enter() {
        let calls = run(
            Os::Macos,
            &[
                Message::Text {
                    value: "he\u{0}llo\n\u{7f}!".into(),
                    submit: true,
                },
                Message::Text {
                    value: "\u{1}\u{2}".into(),
                    submit: false,
                },
                Message::Text {
                    value: "\u{1}".into(),
                    submit: true,
                },
            ],
        );
        assert_eq!(
            calls,
            vec![
                Call::Text("hello!".into()),
                Call::Key(KeyName::Enter),
                Call::Key(KeyName::Enter), // nothing left to type, but submit still presses Enter
            ]
        );
    }

    #[test]
    fn browser_actions_use_cmd_on_macos_and_ctrl_elsewhere() {
        let mac = |a| browser_chord(Os::Macos, a);
        let win = |a| browser_chord(Os::Windows, a);
        assert_eq!(
            mac(BrowserAction::Reload),
            chord(&[Modifier::Cmd], ChordKey::Char('r'))
        );
        assert_eq!(
            win(BrowserAction::Reload),
            chord(&[Modifier::Ctrl], ChordKey::Char('r'))
        );
        assert_eq!(
            mac(BrowserAction::CloseTab),
            chord(&[Modifier::Cmd], ChordKey::Char('w'))
        );
        assert_eq!(
            win(BrowserAction::CloseTab),
            chord(&[Modifier::Ctrl], ChordKey::Char('w'))
        );
        assert_eq!(
            mac(BrowserAction::Back),
            chord(&[Modifier::Cmd], ChordKey::Char('['))
        );
        assert_eq!(
            win(BrowserAction::Back),
            chord(&[Modifier::Alt], ChordKey::Named(KeyName::Left))
        );
        assert_eq!(
            mac(BrowserAction::PrevTab),
            chord(
                &[Modifier::Ctrl, Modifier::Shift],
                ChordKey::Named(KeyName::Tab)
            )
        );
        assert_eq!(
            mac(BrowserAction::NextTab),
            chord(&[Modifier::Ctrl], ChordKey::Named(KeyName::Tab))
        );
        let calls = run(
            Os::Macos,
            &[Message::Browser {
                action: BrowserAction::Forward,
            }],
        );
        assert_eq!(
            calls,
            vec![Call::Chord(chord(&[Modifier::Cmd], ChordKey::Char(']')))]
        );
    }

    #[test]
    fn generic_profile_player_actions() {
        let calls = run(
            Os::Macos,
            &[
                Message::Player {
                    action: PlayerAction::SeekBack,
                },
                Message::Player {
                    action: PlayerAction::SeekFwd,
                },
                Message::Player {
                    action: PlayerAction::Fullscreen,
                },
                Message::Player {
                    action: PlayerAction::Captions,
                },
            ],
        );
        assert_eq!(
            calls,
            vec![
                Call::Chord(chord(&[], ChordKey::Named(KeyName::Left))),
                Call::Chord(chord(&[], ChordKey::Named(KeyName::Right))),
                Call::Chord(chord(&[], ChordKey::Char('f'))),
                Call::Chord(chord(&[], ChordKey::Char('c'))),
            ]
        );
    }

    #[test]
    fn held_buttons_are_released_on_release_all() {
        let mock = MockBackend::new();
        let log = mock.log();
        let ex = Executor::spawn(Box::new(mock), Os::Macos);
        ex.handle(&Message::Button {
            button: MouseButton::Left,
            state: ButtonState::Down,
        });
        ex.handle(&Message::Button {
            button: MouseButton::Middle,
            state: ButtonState::Down,
        });
        ex.handle(&Message::Button {
            button: MouseButton::Middle,
            state: ButtonState::Up,
        });
        ex.release_all();
        ex.flush();
        let calls = log.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![
                Call::Button(MouseButton::Left, ButtonState::Down),
                Call::Button(MouseButton::Middle, ButtonState::Down),
                Call::Button(MouseButton::Middle, ButtonState::Up),
                Call::Button(MouseButton::Left, ButtonState::Up), // only the still-held one
            ]
        );
        ex.release_all();
        ex.flush();
        assert_eq!(
            log.lock().unwrap().len(),
            4,
            "nothing held, nothing released"
        );
    }

    #[test]
    fn dropping_the_executor_releases_held_buttons() {
        let mock = MockBackend::new();
        let log = mock.log();
        {
            let ex = Executor::spawn(Box::new(mock), Os::Macos);
            ex.handle(&Message::Button {
                button: MouseButton::Right,
                state: ButtonState::Down,
            });
            ex.flush();
        }
        let calls = log.lock().unwrap().clone();
        assert_eq!(
            calls.last(),
            Some(&Call::Button(MouseButton::Right, ButtonState::Up))
        );
    }

    #[test]
    fn unknown_profile_and_host_messages_do_nothing() {
        let calls = run(
            Os::Macos,
            &[
                Message::Profile {
                    id: "netflix".into(),
                },
                Message::Ping { t: 1 },
                Message::Error {
                    code: crate::protocol::ErrorCode::Idle,
                },
            ],
        );
        assert!(calls.is_empty());
    }

    /// Backend that blocks until the test lets it go.
    struct Stuck(Arc<Mutex<()>>);
    impl InputBackend for Stuck {
        fn move_rel(&mut self, _: i32, _: i32) -> crate::input::Result<()> {
            let _g = self.0.lock().unwrap();
            Ok(())
        }
        fn button(&mut self, _: MouseButton, _: ButtonState) -> crate::input::Result<()> {
            Ok(())
        }
        fn click(&mut self, _: MouseButton, _: u8) -> crate::input::Result<()> {
            Ok(())
        }
        fn scroll(&mut self, _: i32, _: i32) -> crate::input::Result<()> {
            Ok(())
        }
        fn key(&mut self, _: KeyName) -> crate::input::Result<()> {
            Ok(())
        }
        fn chord(&mut self, _: &Chord) -> crate::input::Result<()> {
            Ok(())
        }
        fn text(&mut self, _: &str) -> crate::input::Result<()> {
            Ok(())
        }
        fn media(&mut self, _: MediaAction) -> crate::input::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_stuck_backend_never_blocks_the_network_side() {
        let gate = Arc::new(Mutex::new(()));
        let held = gate.lock().unwrap();
        let ex = Executor::spawn(Box::new(Stuck(gate.clone())), Os::Macos);
        let t = Instant::now();
        for _ in 0..500 {
            ex.handle(&mv(5.0, 5.0));
            ex.handle(&Message::Key { key: KeyName::Up });
        }
        assert!(
            t.elapsed() < Duration::from_secs(2),
            "handle() must not block"
        );
        drop(held);
        ex.flush(); // the worker recovers and keeps working
    }
}
