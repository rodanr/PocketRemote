//! Manual self-test of the real input backend through the executor (macOS / Windows).
//! Moves the cursor a little and reads its position back. Run: cargo run --example input_selftest
/// The OS applies injected events a few milliseconds after the call returns.
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn settle() {
    std::thread::sleep(std::time::Duration::from_millis(150));
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() {
    use enigo::{Enigo, Mouse, Settings};
    use pocketremote::executor::{Executor, Os};
    use pocketremote::input::{InputError, enigo_backend::EnigoBackend};
    use pocketremote::protocol::Message;
    use pocketremote::server::CommandSink;

    let backend = match EnigoBackend::new() {
        Ok(b) => b,
        Err(InputError::NoPermission) => {
            eprintln!("NO PERMISSION: grant Accessibility to this terminal, then rerun.");
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("backend failed: {e}");
            std::process::exit(1);
        }
    };
    let probe = Enigo::new(&Settings::default()).expect("probe");
    let ex = Executor::spawn(Box::new(backend), Os::current());
    let start = probe.location().unwrap();
    for _ in 0..25 {
        ex.handle(&Message::Move { dx: 4.0, dy: 2.0 });
    }
    ex.flush();
    settle();
    let mid = probe.location().unwrap();
    for _ in 0..25 {
        ex.handle(&Message::Move { dx: -4.0, dy: -2.0 });
    }
    ex.flush();
    settle();
    let end = probe.location().unwrap();
    println!("start {start:?} -> moved {mid:?} -> back {end:?}");
    println!(
        "moved by ({}, {}) (expected (100, 50))",
        mid.0 - start.0,
        mid.1 - start.1
    );
    let ok = (mid.0 - start.0, mid.1 - start.1) == (100, 50) && end == start;
    println!("{}", if ok { "PASS" } else { "FAIL" });
    std::process::exit(if ok { 0 } else { 1 });
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {
    eprintln!("no real input backend on this OS yet");
}
