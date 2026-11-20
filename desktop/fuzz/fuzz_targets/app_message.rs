//! App-message JSON decoding must never panic, and anything it accepts must obey the limits.
#![no_main]
use libfuzzer_sys::fuzz_target;
use pocketremote::protocol::{MAX_TEXT_SCALARS, Message, sanitize_text};

fuzz_target!(|data: &[u8]| {
    if let Ok(msg) = Message::decode(data) {
        // Accepted messages re-encode to something that decodes to the same message.
        assert_eq!(Message::decode(&msg.encode()), Ok(msg.clone()));
        if let Message::Text { value, .. } = &msg {
            assert!(value.chars().count() <= MAX_TEXT_SCALARS);
            let clean = sanitize_text(value);
            assert!(clean.chars().all(|c| !matches!(c as u32, 0x00..=0x1F | 0x7F..=0x9F)));
        }
    }
});
