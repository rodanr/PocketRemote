//! Frames after the handshake must never panic: the first byte picks how many valid frames to
//! send first (so the receiver is at a non-zero nonce), the rest is the frame under test.
#![no_main]
use libfuzzer_sys::fuzz_target;
use pocketremote::noise::{SESSION_PROTOCOL, SessionResponder, generate_static_keypair};
use snow::Builder;

fuzz_target!(|data: &[u8]| {
    let Some((&warmup, frame)) = data.split_first() else { return };
    let host = generate_static_keypair().unwrap();
    let phone = generate_static_keypair().unwrap();
    let mut init = Builder::new(SESSION_PROTOCOL.parse().unwrap())
        .local_private_key(&*phone.private)
        .unwrap()
        .remote_public_key(&host.public)
        .unwrap()
        .build_initiator()
        .unwrap();
    let mut buf = [0u8; 1024];
    let mut scratch = [0u8; 1024];
    let n = init.write_message(&[], &mut buf).unwrap();
    let responder = SessionResponder::new(&host.private, &phone.public).unwrap();
    let (reply, mut host_t) = responder.respond(&buf[..n]).unwrap();
    init.read_message(&reply, &mut scratch).unwrap();
    let mut phone_t = init.into_transport_mode().unwrap();
    for i in 0..(warmup % 4) {
        let mut out = [0u8; 64];
        let m = phone_t.write_message(&[i], &mut out).unwrap();
        assert_eq!(host_t.decrypt(&out[..m]).unwrap(), vec![i]);
    }
    let _ = host_t.encrypt(frame);
    let _ = host_t.decrypt(frame);
});
