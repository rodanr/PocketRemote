//! Prints the current cursor position (macOS / Windows). Used to verify phone input end to end.
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() {
    use enigo::{Enigo, Mouse, Settings};
    let e = Enigo::new(&Settings::default()).expect("input permission?");
    let (x, y) = e.location().expect("location");
    println!("{x} {y}");
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
