//! Frames before the handshake completes must never panic: header parsing, the session
//! responder and every step of the pairing responder, fed arbitrary bytes.
#![no_main]
use libfuzzer_sys::fuzz_target;
use pocketremote::noise::{
    PairingResponder, SessionResponder, generate_static_keypair, split_handshake_frame,
};
use std::sync::OnceLock;

struct Keys {
    host_private: [u8; 32],
    phone_public: [u8; 32],
}

fn keys() -> &'static Keys {
    static KEYS: OnceLock<Keys> = OnceLock::new();
    KEYS.get_or_init(|| {
        let host = generate_static_keypair().unwrap();
        let phone = generate_static_keypair().unwrap();
        Keys { host_private: *host.private, phone_public: phone.public }
    })
}

fuzz_target!(|data: &[u8]| {
    let k = keys();
    let _ = split_handshake_frame(data);
    if let Ok(r) = SessionResponder::new(&k.host_private, &k.phone_public) {
        let _ = r.respond(data);
    }
    if let Ok(mut p) = PairingResponder::new(&k.host_private, &[7u8; 32]) {
        if p.read_msg1(data).is_ok() {
            let _ = p.write_msg2();
            let _ = p.read_msg3(data);
        }
        let _ = p.finish();
    }
});
