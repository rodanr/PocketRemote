//! Pairing window, token and approval (PLAN.md §3.4). Pure state machine: callers pass `now`,
//! so expiry is testable without sleeping.
//!
//! Nothing is stored until the user approves on the host. Never log the token.

use crate::keystore::{Keystore, KeystoreError};
use crate::noise::{PairingResponder, PublicKey};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use zeroize::Zeroizing;

pub const WINDOW: Duration = Duration::from_secs(120);
pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(60);
pub const MAX_FAILED_ATTEMPTS: u8 = 3;
pub const QR_SCHEME: &str = "pocketremote";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approved,
    Rejected,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    NoWindow,
    Expired,
    AttemptInProgress,
    AwaitingApproval,
    Internal,
}

/// What the Control Panel shows while a pairing is waiting for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingApproval {
    /// Six digits, e.g. "123456". Display as "123 456".
    pub code: String,
}

pub struct PairingOffer {
    /// Content of the QR code.
    pub uri: String,
    pub expires_in: Duration,
}

enum State {
    Closed,
    Open {
        token: Zeroizing<[u8; 32]>,
        opened: Instant,
        failures: u8,
        in_flight: bool,
    },
    Awaiting {
        phone_public: PublicKey,
        code: String,
        since: Instant,
        decide: oneshot::Sender<Decision>,
    },
}

pub struct PairingManager {
    state: State,
    /// Counts pairing windows. An attempt that started in an older window must not be able to finish
    /// (or fail) inside a newer one, so attempts carry the generation they began in.
    generation: u64,
}

impl Default for PairingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PairingManager {
    pub fn new() -> Self {
        Self {
            state: State::Closed,
            generation: 0,
        }
    }

    /// The current pairing window's generation; pass it back to `attempt_failed` / `attempt_succeeded`.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Open a pairing window with a fresh token. Any previous window is burned.
    pub fn open(
        &mut self,
        host_public: &PublicKey,
        addr: SocketAddr,
        now: Instant,
    ) -> Result<PairingOffer, Refused> {
        self.close(Decision::Rejected);
        self.generation += 1;
        let mut token = Zeroizing::new([0u8; 32]);
        getrandom::fill(token.as_mut_slice()).map_err(|_| Refused::Internal)?;
        let uri = format!(
            "{QR_SCHEME}://pair?v=1&addr={addr}&pt={}&hk={}&id={}",
            URL_SAFE_NO_PAD.encode(token.as_slice()),
            URL_SAFE_NO_PAD.encode(host_public),
            host_id(host_public),
        );
        self.state = State::Open {
            token,
            opened: now,
            failures: 0,
            in_flight: false,
        };
        Ok(PairingOffer {
            uri,
            expires_in: WINDOW,
        })
    }

    pub fn is_open(&mut self, now: Instant) -> bool {
        self.tick(now);
        matches!(self.state, State::Open { .. })
    }

    pub fn pending(&mut self, now: Instant) -> Option<PendingApproval> {
        self.tick(now);
        match &self.state {
            State::Awaiting { code, .. } => Some(PendingApproval { code: code.clone() }),
            _ => None,
        }
    }

    /// Start one pairing handshake. Only one attempt at a time, only inside the window.
    pub fn begin_attempt(
        &mut self,
        host_private: &[u8; 32],
        now: Instant,
    ) -> Result<PairingResponder, Refused> {
        self.tick(now);
        match &mut self.state {
            State::Closed => Err(Refused::NoWindow),
            State::Awaiting { .. } => Err(Refused::AwaitingApproval),
            State::Open {
                token, in_flight, ..
            } => {
                if *in_flight {
                    return Err(Refused::AttemptInProgress);
                }
                let responder =
                    PairingResponder::new(host_private, token).map_err(|_| Refused::Internal)?;
                *in_flight = true;
                Ok(responder)
            }
        }
    }

    /// The handshake failed. Three failures in one window burn the token and close it.
    pub fn attempt_failed(&mut self, generation: u64, now: Instant) {
        if generation != self.generation {
            return; // a stale attempt from an older window
        }
        self.tick(now);
        if let State::Open {
            failures,
            in_flight,
            ..
        } = &mut self.state
        {
            *in_flight = false;
            *failures += 1;
            if *failures >= MAX_FAILED_ATTEMPTS {
                self.state = State::Closed;
            }
        }
    }

    /// The handshake succeeded: the token is burned now and the user must approve.
    pub fn attempt_succeeded(
        &mut self,
        generation: u64,
        phone_public: PublicKey,
        code: String,
        now: Instant,
    ) -> Result<oneshot::Receiver<Decision>, Refused> {
        if generation != self.generation {
            return Err(Refused::NoWindow); // started in an older window
        }
        self.tick(now);
        match self.state {
            State::Open {
                in_flight: true, ..
            } => {
                let (tx, rx) = oneshot::channel();
                self.state = State::Awaiting {
                    phone_public,
                    code,
                    since: now,
                    decide: tx,
                };
                Ok(rx)
            }
            _ => Err(Refused::NoWindow),
        }
    }

    /// The user clicked Approve. Stores the phone, replacing and so revoking any previous one.
    pub fn approve(&mut self, keystore: &Keystore, now: Instant) -> Result<(), ApproveError> {
        self.tick(now);
        match std::mem::replace(&mut self.state, State::Closed) {
            State::Awaiting {
                phone_public,
                decide,
                ..
            } => {
                if let Err(e) = keystore.set_paired_phone(&phone_public) {
                    let _ = decide.send(Decision::Rejected);
                    return Err(ApproveError::Store(e));
                }
                let _ = decide.send(Decision::Approved);
                Ok(())
            }
            other => {
                self.state = other;
                Err(ApproveError::NothingPending)
            }
        }
    }

    /// The user clicked Reject, or closed the pairing window. Nothing is stored.
    pub fn reject(&mut self) {
        self.close(Decision::Rejected);
    }

    /// Expire the window or a waiting approval.
    pub fn tick(&mut self, now: Instant) {
        let expired = match &self.state {
            State::Open { opened, .. } => now.saturating_duration_since(*opened) >= WINDOW,
            State::Awaiting { since, .. } => {
                now.saturating_duration_since(*since) >= APPROVAL_TIMEOUT
            }
            State::Closed => false,
        };
        if expired {
            self.close(Decision::TimedOut);
        }
    }

    fn close(&mut self, why: Decision) {
        if let State::Awaiting { decide, .. } = std::mem::replace(&mut self.state, State::Closed) {
            let _ = decide.send(why);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproveError {
    NothingPending,
    Store(KeystoreError),
}

/// Short, stable, non-secret identifier derived from the host public key.
fn host_id(host_public: &PublicKey) -> String {
    host_public[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keystore::MemoryStore;
    use crate::noise::{PAIRING_PROTOCOL, generate_static_keypair, pairing_code};
    use snow::Builder;

    fn addr() -> SocketAddr {
        "192.168.1.20:9876".parse().unwrap()
    }

    fn ks() -> Keystore {
        Keystore::new(Box::new(MemoryStore::default()))
    }

    fn token_from_uri(uri: &str) -> [u8; 32] {
        let pt = uri
            .split(['?', '&'])
            .find_map(|kv| kv.strip_prefix("pt="))
            .unwrap();
        URL_SAFE_NO_PAD.decode(pt).unwrap().try_into().unwrap()
    }

    /// Run a full pairing handshake as the phone. Returns (phone key, phone-side code).
    fn run_handshake(
        mgr: &mut PairingManager,
        host: &crate::noise::StaticKeypair,
        token: &[u8; 32],
        now: Instant,
    ) -> Result<(PublicKey, String, oneshot::Receiver<Decision>), Refused> {
        let phone = generate_static_keypair().unwrap();
        let mut resp = mgr.begin_attempt(&host.private, now)?;
        let mut init = Builder::new(PAIRING_PROTOCOL.parse().unwrap())
            .local_private_key(&*phone.private)
            .unwrap()
            .psk(3, token)
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut buf = [0u8; 512];
        let mut p = [0u8; 512];
        let n = init.write_message(&[], &mut buf).unwrap();
        resp.read_msg1(&buf[..n]).unwrap();
        let m2 = resp.write_msg2().unwrap();
        init.read_message(&m2, &mut p).unwrap();
        let n = init.write_message(&[], &mut buf).unwrap();
        resp.read_msg3(&buf[..n]).unwrap();
        let phone_code = pairing_code(init.get_handshake_hash());
        let out = resp.finish().unwrap();
        assert_eq!(out.code, phone_code);
        let rx = mgr.attempt_succeeded(mgr.generation(), out.phone_public, out.code, now)?;
        Ok((phone.public, phone_code, rx))
    }

    #[test]
    fn uri_has_the_documented_shape() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let offer = mgr.open(&host.public, addr(), Instant::now()).unwrap();
        assert!(
            offer
                .uri
                .starts_with("pocketremote://pair?v=1&addr=192.168.1.20:9876&pt=")
        );
        assert!(offer.uri.contains("&hk="));
        assert!(offer.uri.contains("&id="));
        assert_eq!(offer.expires_in, Duration::from_secs(120));
    }

    #[test]
    fn no_attempt_without_a_window() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        assert!(matches!(
            mgr.begin_attempt(&host.private, Instant::now()),
            Err(Refused::NoWindow)
        ));
    }

    #[test]
    fn window_expires_after_120_seconds() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        mgr.open(&host.public, addr(), t0).unwrap();
        assert!(mgr.is_open(t0 + Duration::from_secs(119)));
        assert!(!mgr.is_open(t0 + Duration::from_secs(120)));
        assert!(matches!(
            mgr.begin_attempt(&host.private, t0 + Duration::from_secs(121)),
            Err(Refused::NoWindow)
        ));
    }

    #[test]
    fn only_one_attempt_at_a_time() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        mgr.open(&host.public, addr(), t0).unwrap();
        let _first = mgr.begin_attempt(&host.private, t0).unwrap();
        assert!(matches!(
            mgr.begin_attempt(&host.private, t0),
            Err(Refused::AttemptInProgress)
        ));
    }

    #[test]
    fn an_attempt_from_an_older_window_cannot_finish_or_fail_in_a_new_one() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        mgr.open(&host.public, addr(), t0).unwrap();
        let _ = mgr.begin_attempt(&host.private, t0).unwrap();
        let old = mgr.generation();
        // The user clicks Pair again: a new window opens while the old attempt is still running.
        mgr.open(&host.public, addr(), t0).unwrap();
        let _ = mgr.begin_attempt(&host.private, t0).unwrap(); // the new window's own attempt
        assert!(matches!(
            mgr.attempt_succeeded(old, [7; 32], "123456".into(), t0),
            Err(Refused::NoWindow)
        ));
        mgr.attempt_failed(old, t0); // ignored: must not count against the new window
        assert!(
            matches!(
                mgr.begin_attempt(&host.private, t0),
                Err(Refused::AttemptInProgress)
            ),
            "the new window's attempt is still the one in flight"
        );
    }

    #[test]
    fn three_failed_attempts_close_the_window() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        mgr.open(&host.public, addr(), t0).unwrap();
        for _ in 0..2 {
            let _ = mgr.begin_attempt(&host.private, t0).unwrap();
            mgr.attempt_failed(mgr.generation(), t0);
            assert!(mgr.is_open(t0));
        }
        let _ = mgr.begin_attempt(&host.private, t0).unwrap();
        mgr.attempt_failed(mgr.generation(), t0);
        assert!(!mgr.is_open(t0));
    }

    #[test]
    fn approve_stores_phone_and_burns_token() {
        let host = generate_static_keypair().unwrap();
        let store = ks();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        let offer = mgr.open(&host.public, addr(), t0).unwrap();
        let token = token_from_uri(&offer.uri);
        let (phone_pub, code, mut rx) = run_handshake(&mut mgr, &host, &token, t0).unwrap();
        assert_eq!(mgr.pending(t0).unwrap().code, code);
        assert_eq!(
            store.paired_phone().unwrap(),
            None,
            "nothing stored before Approve"
        );
        mgr.approve(&store, t0).unwrap();
        assert_eq!(rx.try_recv().unwrap(), Decision::Approved);
        assert_eq!(store.paired_phone().unwrap(), Some(phone_pub));
        // token reuse is refused: the window is closed
        assert!(matches!(
            mgr.begin_attempt(&host.private, t0),
            Err(Refused::NoWindow)
        ));
    }

    #[test]
    fn reject_stores_nothing() {
        let host = generate_static_keypair().unwrap();
        let store = ks();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        let offer = mgr.open(&host.public, addr(), t0).unwrap();
        let token = token_from_uri(&offer.uri);
        let (_, _, mut rx) = run_handshake(&mut mgr, &host, &token, t0).unwrap();
        mgr.reject();
        assert_eq!(rx.try_recv().unwrap(), Decision::Rejected);
        assert_eq!(store.paired_phone().unwrap(), None);
        assert!(mgr.pending(t0).is_none());
        assert!(matches!(
            mgr.approve(&store, t0),
            Err(ApproveError::NothingPending)
        ));
    }

    #[test]
    fn unanswered_approval_times_out_and_stores_nothing() {
        let host = generate_static_keypair().unwrap();
        let store = ks();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        let offer = mgr.open(&host.public, addr(), t0).unwrap();
        let token = token_from_uri(&offer.uri);
        let (_, _, mut rx) = run_handshake(&mut mgr, &host, &token, t0).unwrap();
        let later = t0 + APPROVAL_TIMEOUT;
        assert!(mgr.pending(later).is_none());
        assert_eq!(rx.try_recv().unwrap(), Decision::TimedOut);
        assert!(matches!(
            mgr.approve(&store, later),
            Err(ApproveError::NothingPending)
        ));
        assert_eq!(store.paired_phone().unwrap(), None);
    }

    #[test]
    fn approving_a_new_phone_replaces_the_old_one() {
        let host = generate_static_keypair().unwrap();
        let store = ks();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        let mut keys = Vec::new();
        for _ in 0..2 {
            let offer = mgr.open(&host.public, addr(), t0).unwrap();
            let token = token_from_uri(&offer.uri);
            let (phone_pub, _, _rx) = run_handshake(&mut mgr, &host, &token, t0).unwrap();
            mgr.approve(&store, t0).unwrap();
            keys.push(phone_pub);
        }
        assert_ne!(keys[0], keys[1]);
        assert_eq!(store.paired_phone().unwrap(), Some(keys[1]));
    }

    #[test]
    fn wrong_token_fails_and_is_counted() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        mgr.open(&host.public, addr(), t0).unwrap();
        let phone = generate_static_keypair().unwrap();
        let mut resp = mgr.begin_attempt(&host.private, t0).unwrap();
        let mut init = Builder::new(PAIRING_PROTOCOL.parse().unwrap())
            .local_private_key(&*phone.private)
            .unwrap()
            .psk(3, &[0xAA; 32])
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut buf = [0u8; 512];
        let mut p = [0u8; 512];
        let n = init.write_message(&[], &mut buf).unwrap();
        resp.read_msg1(&buf[..n]).unwrap();
        let m2 = resp.write_msg2().unwrap();
        init.read_message(&m2, &mut p).unwrap();
        let n = init.write_message(&[], &mut buf).unwrap();
        assert!(resp.read_msg3(&buf[..n]).is_err());
        mgr.attempt_failed(mgr.generation(), t0);
        assert!(mgr.is_open(t0), "one failure does not close the window");
    }

    #[test]
    fn reopening_burns_the_old_token() {
        let host = generate_static_keypair().unwrap();
        let mut mgr = PairingManager::new();
        let t0 = Instant::now();
        let first = mgr.open(&host.public, addr(), t0).unwrap();
        let second = mgr.open(&host.public, addr(), t0).unwrap();
        let old = token_from_uri(&first.uri);
        assert_ne!(old, token_from_uri(&second.uri));
        // A phone that scanned the first QR cannot finish the handshake any more.
        let phone = generate_static_keypair().unwrap();
        let mut resp = mgr.begin_attempt(&host.private, t0).unwrap();
        let mut init = Builder::new(PAIRING_PROTOCOL.parse().unwrap())
            .local_private_key(&*phone.private)
            .unwrap()
            .psk(3, &old)
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut buf = [0u8; 512];
        let mut p = [0u8; 512];
        let n = init.write_message(&[], &mut buf).unwrap();
        resp.read_msg1(&buf[..n]).unwrap();
        let m2 = resp.write_msg2().unwrap();
        init.read_message(&m2, &mut p).unwrap();
        let n = init.write_message(&[], &mut buf).unwrap();
        assert!(resp.read_msg3(&buf[..n]).is_err());
    }
}
