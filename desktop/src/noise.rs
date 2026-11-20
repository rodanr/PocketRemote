//! Noise wrappers (PLAN.md §3.2, §3.4, §3.5, §4.1). Thin layer over `snow`: no custom crypto.
//!
//! * Pairing: `Noise_XXpsk3_25519_ChaChaPoly_SHA256`, PSK = the 32-byte pairing token.
//! * Sessions: `Noise_KK_25519_ChaChaPoly_SHA256`, both static keys known from pairing.
//!
//! Errors are deliberately opaque (`NoiseError`): callers close the socket without detail.
//! Nothing here logs. Never add logging of keys, tokens or payloads.

use snow::{Builder, HandshakeState, TransportState};
use std::fmt;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub const SESSION_PROTOCOL: &str = "Noise_KK_25519_ChaChaPoly_SHA256";
pub const PAIRING_PROTOCOL: &str = "Noise_XXpsk3_25519_ChaChaPoly_SHA256";
pub const HANDSHAKE_VERSION: u8 = 0x01;
/// Handshake frames (header included) are at most this many bytes (PLAN.md §3.9).
pub const MAX_HANDSHAKE_FRAME: usize = 512;
/// Post-handshake frames are at most this many bytes (PLAN.md §3.9).
pub const MAX_TRANSPORT_FRAME: usize = 4096;
const TAG_LEN: usize = 16;
/// Rekey a direction after this many messages (PLAN.md §4.1).
pub const REKEY_INTERVAL: u64 = 1 << 20;

pub type PublicKey = [u8; 32];

/// Opaque failure. Carries no detail on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseError;

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("noise failure")
    }
}

impl std::error::Error for NoiseError {}

impl From<snow::Error> for NoiseError {
    fn from(_: snow::Error) -> Self {
        Self
    }
}

type Result<T> = std::result::Result<T, NoiseError>;

/// A static X25519 keypair. The private half is zeroized on drop.
pub struct StaticKeypair {
    pub private: Zeroizing<[u8; 32]>,
    pub public: PublicKey,
}

pub fn generate_static_keypair() -> Result<StaticKeypair> {
    let kp = Builder::new(SESSION_PROTOCOL.parse()?).generate_keypair()?;
    let mut private = Zeroizing::new([0u8; 32]);
    private.copy_from_slice(&kp.private);
    let public = kp.public.as_slice().try_into().map_err(|_| NoiseError)?;
    Ok(StaticKeypair { private, public })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeKind {
    Pairing = 0x01,
    Session = 0x02,
}

/// Split a handshake frame `[kind][version][noise message]`. Unknown kind or version, or a
/// frame over 512 bytes, is an error and the caller must close the socket.
pub fn split_handshake_frame(frame: &[u8]) -> Result<(HandshakeKind, &[u8])> {
    if frame.len() > MAX_HANDSHAKE_FRAME || frame.len() < 3 {
        return Err(NoiseError);
    }
    if frame[1] != HANDSHAKE_VERSION {
        return Err(NoiseError);
    }
    let kind = match frame[0] {
        0x01 => HandshakeKind::Pairing,
        0x02 => HandshakeKind::Session,
        _ => return Err(NoiseError),
    };
    Ok((kind, &frame[2..]))
}

pub fn encode_handshake_frame(kind: HandshakeKind, noise_message: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + noise_message.len());
    out.push(kind as u8);
    out.push(HANDSHAKE_VERSION);
    out.extend_from_slice(noise_message);
    out
}

fn read_empty_payload(hs: &mut HandshakeState, msg: &[u8]) -> Result<()> {
    let mut payload = [0u8; MAX_HANDSHAKE_FRAME];
    let n = hs.read_message(msg, &mut payload)?;
    // Handshake payloads are unused: anything non-empty is a protocol violation.
    if n == 0 { Ok(()) } else { Err(NoiseError) }
}

fn write_empty_payload(hs: &mut HandshakeState) -> Result<Vec<u8>> {
    let mut buf = [0u8; MAX_HANDSHAKE_FRAME];
    let n = hs.write_message(&[], &mut buf)?;
    Ok(buf[..n].to_vec())
}

/// Encrypted transport after a handshake. One Noise transport message per frame.
pub struct Transport {
    inner: TransportState,
    sent: u64,
    received: u64,
    rekey_interval: u64,
}

impl Transport {
    fn new(inner: TransportState) -> Self {
        Self::with_rekey_interval(inner, REKEY_INTERVAL)
    }

    pub(crate) fn with_rekey_interval(inner: TransportState, rekey_interval: u64) -> Self {
        Self {
            inner,
            sent: 0,
            received: 0,
            rekey_interval,
        }
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if plaintext.len() + TAG_LEN > MAX_TRANSPORT_FRAME {
            return Err(NoiseError);
        }
        let mut out = vec![0u8; plaintext.len() + TAG_LEN];
        let n = self.inner.write_message(plaintext, &mut out)?;
        out.truncate(n);
        self.sent += 1;
        if self.sent == self.rekey_interval {
            self.inner.rekey_outgoing();
            self.sent = 0;
        }
        Ok(out)
    }

    /// Any failure (bad tag, replay, reorder, oversize) must close the session.
    pub fn decrypt(&mut self, frame: &[u8]) -> Result<Vec<u8>> {
        if frame.len() > MAX_TRANSPORT_FRAME || frame.len() < TAG_LEN {
            return Err(NoiseError);
        }
        let mut out = vec![0u8; frame.len()];
        let n = self.inner.read_message(frame, &mut out)?;
        out.truncate(n);
        self.received += 1;
        if self.received == self.rekey_interval {
            self.inner.rekey_incoming();
            self.received = 0;
        }
        Ok(out)
    }
}

/// Host side of a session handshake (`KK`). Both static keys are known from pairing.
pub struct SessionResponder {
    hs: HandshakeState,
    phone_public: PublicKey,
}

impl SessionResponder {
    pub fn new(host_private: &[u8; 32], phone_public: &PublicKey) -> Result<Self> {
        let hs = Builder::new(SESSION_PROTOCOL.parse()?)
            .local_private_key(host_private)?
            .remote_public_key(phone_public)?
            .build_responder()?;
        Ok(Self {
            hs,
            phone_public: *phone_public,
        })
    }

    /// Consume the phone's first message; return the reply message and the transport.
    pub fn respond(mut self, first_message: &[u8]) -> Result<(Vec<u8>, Transport)> {
        read_empty_payload(&mut self.hs, first_message)?;
        let reply = write_empty_payload(&mut self.hs)?;
        if !self.hs.is_handshake_finished() {
            return Err(NoiseError);
        }
        let remote = self.hs.get_remote_static().ok_or(NoiseError)?;
        if !bool::from(remote.ct_eq(&self.phone_public)) {
            return Err(NoiseError);
        }
        Ok((reply, Transport::new(self.hs.into_transport_mode()?)))
    }
}

/// Result of a completed pairing handshake. Nothing is stored until the user approves.
pub struct PairingOutcome {
    pub phone_public: PublicKey,
    /// Six digits, from the handshake hash. Shown on both devices.
    pub code: String,
    /// Used to send `paired` after the user approves.
    pub transport: Transport,
}

#[derive(Debug, PartialEq, Eq)]
enum PairingStep {
    ExpectMsg1,
    WriteMsg2,
    ExpectMsg3,
    Done,
}

/// Host side of the pairing handshake (`XXpsk3`).
pub struct PairingResponder {
    hs: HandshakeState,
    step: PairingStep,
}

impl PairingResponder {
    pub fn new(host_private: &[u8; 32], pairing_token: &[u8; 32]) -> Result<Self> {
        let hs = Builder::new(PAIRING_PROTOCOL.parse()?)
            .local_private_key(host_private)?
            .psk(3, pairing_token)?
            .build_responder()?;
        Ok(Self {
            hs,
            step: PairingStep::ExpectMsg1,
        })
    }

    /// `-> e`
    pub fn read_msg1(&mut self, msg: &[u8]) -> Result<()> {
        if self.step != PairingStep::ExpectMsg1 {
            return Err(NoiseError);
        }
        read_empty_payload(&mut self.hs, msg)?;
        self.step = PairingStep::WriteMsg2;
        Ok(())
    }

    /// `<- e, ee, s, es`
    pub fn write_msg2(&mut self) -> Result<Vec<u8>> {
        if self.step != PairingStep::WriteMsg2 {
            return Err(NoiseError);
        }
        let msg = write_empty_payload(&mut self.hs)?;
        self.step = PairingStep::ExpectMsg3;
        Ok(msg)
    }

    /// `-> s, se, psk`. A wrong pairing token fails here.
    pub fn read_msg3(&mut self, msg: &[u8]) -> Result<()> {
        if self.step != PairingStep::ExpectMsg3 {
            return Err(NoiseError);
        }
        read_empty_payload(&mut self.hs, msg)?;
        self.step = PairingStep::Done;
        Ok(())
    }

    pub fn finish(self) -> Result<PairingOutcome> {
        if self.step != PairingStep::Done || !self.hs.is_handshake_finished() {
            return Err(NoiseError);
        }
        let phone_public: PublicKey = self
            .hs
            .get_remote_static()
            .ok_or(NoiseError)?
            .try_into()
            .map_err(|_| NoiseError)?;
        let code = pairing_code(self.hs.get_handshake_hash());
        let transport = Transport::new(self.hs.into_transport_mode()?);
        Ok(PairingOutcome {
            phone_public,
            code,
            transport,
        })
    }
}

/// First 4 bytes of the handshake hash, big-endian, mod 1,000,000, zero-padded (PLAN.md §3.4).
pub fn pairing_code(handshake_hash: &[u8]) -> String {
    let mut first = [0u8; 4];
    first.copy_from_slice(&handshake_hash[..4]);
    format!("{:06}", u32::from_be_bytes(first) % 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pair {
        host: StaticKeypair,
        phone: StaticKeypair,
    }

    fn keys() -> Pair {
        Pair {
            host: generate_static_keypair().unwrap(),
            phone: generate_static_keypair().unwrap(),
        }
    }

    /// Test-only KK initiator built directly on snow.
    fn kk_initiator(phone_private: &[u8; 32], host_public: &PublicKey) -> HandshakeState {
        Builder::new(SESSION_PROTOCOL.parse().unwrap())
            .local_private_key(phone_private)
            .unwrap()
            .remote_public_key(host_public)
            .unwrap()
            .build_initiator()
            .unwrap()
    }

    fn kk_handshake(p: &Pair) -> (Transport, Transport) {
        let mut init = kk_initiator(&p.phone.private, &p.host.public);
        let m1 = write_empty_payload(&mut init).unwrap();
        let resp = SessionResponder::new(&p.host.private, &p.phone.public).unwrap();
        let (m2, host_t) = resp.respond(&m1).unwrap();
        read_empty_payload(&mut init, &m2).unwrap();
        let phone_t = Transport::new(init.into_transport_mode().unwrap());
        (phone_t, host_t)
    }

    #[test]
    fn session_round_trip_both_directions() {
        let p = keys();
        let (mut phone, mut host) = kk_handshake(&p);
        let f = phone.encrypt(b"hello host").unwrap();
        assert_eq!(host.decrypt(&f).unwrap(), b"hello host");
        let f = host.encrypt(b"hello phone").unwrap();
        assert_eq!(phone.decrypt(&f).unwrap(), b"hello phone");
    }

    #[test]
    fn session_rejects_unknown_phone_key() {
        let p = keys();
        let stranger = generate_static_keypair().unwrap();
        let mut init = kk_initiator(&stranger.private, &p.host.public);
        let m1 = write_empty_payload(&mut init).unwrap();
        let resp = SessionResponder::new(&p.host.private, &p.phone.public).unwrap();
        assert!(resp.respond(&m1).is_err());
    }

    #[test]
    fn session_rejects_wrong_pinned_host_key() {
        let p = keys();
        let other_host = generate_static_keypair().unwrap();
        let mut init = kk_initiator(&p.phone.private, &other_host.public);
        let m1 = write_empty_payload(&mut init).unwrap();
        let resp = SessionResponder::new(&p.host.private, &p.phone.public).unwrap();
        assert!(resp.respond(&m1).is_err());
    }

    #[test]
    fn session_rejects_garbage_and_truncated_first_message() {
        let p = keys();
        for bad in [vec![], vec![0u8; 10], vec![0xff; 48], vec![1u8; 200]] {
            let resp = SessionResponder::new(&p.host.private, &p.phone.public).unwrap();
            assert!(resp.respond(&bad).is_err());
        }
    }

    #[test]
    fn tampered_frame_is_rejected() {
        let p = keys();
        let (mut phone, mut host) = kk_handshake(&p);
        let mut f = phone.encrypt(b"move").unwrap();
        f[0] ^= 1;
        assert!(host.decrypt(&f).is_err());
    }

    #[test]
    fn replayed_frame_is_rejected() {
        let p = keys();
        let (mut phone, mut host) = kk_handshake(&p);
        let f = phone.encrypt(b"click").unwrap();
        assert!(host.decrypt(&f).is_ok());
        assert!(host.decrypt(&f).is_err());
    }

    #[test]
    fn out_of_order_frame_is_rejected() {
        let p = keys();
        let (mut phone, mut host) = kk_handshake(&p);
        let _first = phone.encrypt(b"one").unwrap();
        let second = phone.encrypt(b"two").unwrap();
        assert!(host.decrypt(&second).is_err());
    }

    #[test]
    fn size_limits_are_enforced() {
        let p = keys();
        let (mut phone, mut host) = kk_handshake(&p);
        assert!(phone.encrypt(&vec![0u8; MAX_TRANSPORT_FRAME]).is_err());
        assert!(
            phone
                .encrypt(&vec![0u8; MAX_TRANSPORT_FRAME - TAG_LEN])
                .is_ok()
        );
        assert!(host.decrypt(&vec![0u8; MAX_TRANSPORT_FRAME + 1]).is_err());
        assert!(host.decrypt(&[0u8; 3]).is_err());
    }

    #[test]
    fn rekey_keeps_both_sides_in_step() {
        let p = keys();
        let mut init = kk_initiator(&p.phone.private, &p.host.public);
        let m1 = write_empty_payload(&mut init).unwrap();
        let resp = SessionResponder::new(&p.host.private, &p.phone.public).unwrap();
        let (m2, mut host) = resp.respond(&m1).unwrap();
        read_empty_payload(&mut init, &m2).unwrap();
        let mut phone = Transport::with_rekey_interval(init.into_transport_mode().unwrap(), 3);
        host.rekey_interval = 3;
        for i in 0..20u8 {
            let f = phone.encrypt(&[i]).unwrap();
            assert_eq!(host.decrypt(&f).unwrap(), vec![i]);
            let f = host.encrypt(&[i, i]).unwrap();
            assert_eq!(phone.decrypt(&f).unwrap(), vec![i, i]);
        }
    }

    #[test]
    fn rekey_mismatch_breaks_the_session() {
        let p = keys();
        let mut init = kk_initiator(&p.phone.private, &p.host.public);
        let m1 = write_empty_payload(&mut init).unwrap();
        let resp = SessionResponder::new(&p.host.private, &p.phone.public).unwrap();
        let (m2, mut host) = resp.respond(&m1).unwrap();
        read_empty_payload(&mut init, &m2).unwrap();
        let mut phone = Transport::with_rekey_interval(init.into_transport_mode().unwrap(), 2);
        // host keeps the default interval, so it never rekeys
        let _ = host.decrypt(&phone.encrypt(b"a").unwrap());
        let _ = host.decrypt(&phone.encrypt(b"b").unwrap());
        assert!(host.decrypt(&phone.encrypt(b"c").unwrap()).is_err());
    }

    #[test]
    fn handshake_frame_header_is_strict() {
        let ok = encode_handshake_frame(HandshakeKind::Session, &[9; 48]);
        assert_eq!(
            split_handshake_frame(&ok).unwrap().0,
            HandshakeKind::Session
        );
        let ok = encode_handshake_frame(HandshakeKind::Pairing, &[9; 32]);
        assert_eq!(
            split_handshake_frame(&ok).unwrap().0,
            HandshakeKind::Pairing
        );
        assert!(
            split_handshake_frame(&[0x03, 0x01, 1, 2]).is_err(),
            "unknown kind"
        );
        assert!(
            split_handshake_frame(&[0x02, 0x02, 1, 2]).is_err(),
            "unknown version"
        );
        assert!(split_handshake_frame(&[0x02, 0x01]).is_err(), "no body");
        assert!(split_handshake_frame(&[]).is_err());
        let mut big = vec![0x02, 0x01];
        big.resize(MAX_HANDSHAKE_FRAME + 1, 0);
        assert!(split_handshake_frame(&big).is_err(), "over 512 bytes");
        big.truncate(MAX_HANDSHAKE_FRAME);
        assert!(split_handshake_frame(&big).is_ok(), "exactly 512 bytes");
    }

    fn xx_initiator(phone_private: &[u8; 32], token: &[u8; 32]) -> HandshakeState {
        Builder::new(PAIRING_PROTOCOL.parse().unwrap())
            .local_private_key(phone_private)
            .unwrap()
            .psk(3, token)
            .unwrap()
            .build_initiator()
            .unwrap()
    }

    #[test]
    fn pairing_flow_agrees_on_code_and_keys() {
        let p = keys();
        let token = [7u8; 32];
        let mut init = xx_initiator(&p.phone.private, &token);
        let mut host = PairingResponder::new(&p.host.private, &token).unwrap();
        host.read_msg1(&write_empty_payload(&mut init).unwrap())
            .unwrap();
        read_empty_payload(&mut init, &host.write_msg2().unwrap()).unwrap();
        // the phone checks the host key against the QR here
        assert_eq!(init.get_remote_static().unwrap(), p.host.public);
        host.read_msg3(&write_empty_payload(&mut init).unwrap())
            .unwrap();
        let phone_code = pairing_code(init.get_handshake_hash());
        let mut phone_t = Transport::new(init.into_transport_mode().unwrap());
        let mut out = host.finish().unwrap();
        assert_eq!(out.phone_public, p.phone.public);
        assert_eq!(out.code, phone_code);
        assert_eq!(out.code.len(), 6);
        assert!(out.code.bytes().all(|b| b.is_ascii_digit()));
        let f = out.transport.encrypt(b"paired").unwrap();
        assert_eq!(phone_t.decrypt(&f).unwrap(), b"paired");
    }

    #[test]
    fn pairing_with_wrong_token_fails() {
        let p = keys();
        let mut init = xx_initiator(&p.phone.private, &[1u8; 32]);
        let mut host = PairingResponder::new(&p.host.private, &[2u8; 32]).unwrap();
        host.read_msg1(&write_empty_payload(&mut init).unwrap())
            .unwrap();
        // msg2 already depends on the psk only at msg3 (psk3), so msg3 must fail
        let m2 = host.write_msg2().unwrap();
        let _ = read_empty_payload(&mut init, &m2);
        let m3 = write_empty_payload(&mut init);
        if let Ok(m3) = m3 {
            assert!(host.read_msg3(&m3).is_err());
        }
    }

    #[test]
    fn pairing_steps_must_be_in_order() {
        let p = keys();
        let mut host = PairingResponder::new(&p.host.private, &[0u8; 32]).unwrap();
        assert!(host.write_msg2().is_err());
        assert!(host.read_msg3(&[0; 64]).is_err());
        assert!(
            PairingResponder::new(&p.host.private, &[0u8; 32])
                .unwrap()
                .finish()
                .is_err()
        );
    }

    #[test]
    fn pairing_code_formula() {
        assert_eq!(pairing_code(&[0, 0, 0, 0, 9, 9]), "000000");
        assert_eq!(pairing_code(&[0, 0, 0, 1, 9]), "000001");
        // 0x00_0F_42_40 = 1_000_000 -> 0 ; 0x00_0F_42_41 -> 1
        assert_eq!(pairing_code(&[0x00, 0x0F, 0x42, 0x40]), "000000");
        assert_eq!(pairing_code(&[0x00, 0x0F, 0x42, 0x41]), "000001");
        assert_eq!(pairing_code(&[0xFF, 0xFF, 0xFF, 0xFF]), "967295");
    }
}
