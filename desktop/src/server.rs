//! WebSocket server (PLAN.md §3.5, §3.6, §3.9, §4.1).
//!
//! Serves only `GET /ws`; every other request gets an empty 404. Handles pairing and session
//! handshakes, enforces all connection and message limits, and forwards input commands to a
//! [`CommandSink`]. Never logs keys, tokens, payloads or typed text.

use crate::keystore::Keystore;
use crate::net::{self, ConnGuard, Limiter, RateWindow};
use crate::noise::{
    self, HandshakeKind, MAX_TRANSPORT_FRAME, PairingOutcome, SessionResponder, Transport,
};
use crate::pairing::{self, Decision, PairingManager, PairingOffer};
use crate::protocol::{ErrorCode, Message, Platform};
use futures_util::{SinkExt, StreamExt};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant as StdInstant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep, sleep_until, timeout};
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{WebSocketStream, accept_async_with_config};

pub const DEFAULT_PORT: u16 = 9876;
const PORT_ATTEMPTS: u16 = 100;
const MAX_HTTP_HEAD: usize = 4096;
pub const MIN_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const NOT_FOUND: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

type Ws2 = WebSocketStream<TcpStream>;

/// Receives validated phone commands. The executor implements this.
pub trait CommandSink: Send + Sync + 'static {
    /// Called for every input command (`move`, `click`, `key`, `text`, ...).
    fn handle(&self, msg: &Message);
    /// Called when a session ends for any reason: release held mouse buttons and keys.
    fn release_all(&self);
}

/// Things the UI cares about (tray state, notifications, approve dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    PhoneConnected,
    PhoneDisconnected,
    PairingAttempt,
    /// Show "Pair your phone? Code 123 456 — Approve / Reject".
    ApprovalRequested {
        code: String,
    },
    PairingFinished {
        approved: bool,
    },
}

#[derive(Debug)]
pub enum StartError {
    /// Refuses `0.0.0.0`, `::` and any address outside the allowlist.
    UnsafeBind,
    Bind(std::io::Error),
    Keystore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairError {
    Paused,
    Keystore,
    Internal,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub bind_ip: IpAddr,
    pub port: u16,
    pub host_name: String,
    pub platform: Platform,
    pub handshake_timeout: Duration,
    pub silence_timeout: Duration,
    idle_timeout: Duration,
}

impl ServerConfig {
    pub fn new(bind_ip: IpAddr, host_name: String, platform: Platform) -> Self {
        Self {
            bind_ip,
            port: DEFAULT_PORT,
            host_name,
            platform,
            handshake_timeout: Duration::from_secs(3),
            silence_timeout: Duration::from_secs(15),
            idle_timeout: Duration::from_secs(30 * 60),
        }
    }

    /// Idle timeout for "no input commands"; clamped to at least 5 minutes (PLAN.md §3.5).
    pub fn set_idle_timeout(&mut self, d: Duration) {
        self.idle_timeout = d.max(MIN_IDLE_TIMEOUT);
    }
}

enum Control {
    Close(Option<ErrorCode>),
}

struct ActiveSession {
    id: u64,
    ctl: mpsc::UnboundedSender<Control>,
}

struct Shared {
    cfg: ServerConfig,
    bound: SocketAddr,
    keystore: Mutex<Keystore>,
    pairing: Mutex<PairingManager>,
    limiter: Limiter,
    sink: Arc<dyn CommandSink>,
    paused: AtomicBool,
    events: mpsc::UnboundedSender<ServerEvent>,
    active: Mutex<Option<ActiveSession>>,
    next_id: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Handle to a running server. Dropping it stops the accept loop.
pub struct Server {
    shared: Arc<Shared>,
    accept: JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.accept.abort();
    }
}

impl Server {
    /// Bind to the selected interface only and start accepting. The server starts paused when
    /// `start_paused` is set (e.g. a network the user has not yet trusted).
    pub async fn start(
        cfg: ServerConfig,
        keystore: Keystore,
        sink: Arc<dyn CommandSink>,
        events: mpsc::UnboundedSender<ServerEvent>,
        start_paused: bool,
    ) -> Result<Self, StartError> {
        if cfg.bind_ip.is_unspecified() || !net::ip_allowed(cfg.bind_ip) {
            return Err(StartError::UnsafeBind);
        }
        keystore.host_keypair().map_err(|_| StartError::Keystore)?;
        let (listener, bound) = bind_with_fallback(cfg.bind_ip, cfg.port)
            .await
            .map_err(StartError::Bind)?;
        let shared = Arc::new(Shared {
            cfg,
            bound,
            keystore: Mutex::new(keystore),
            pairing: Mutex::new(PairingManager::new()),
            limiter: Limiter::new(),
            sink,
            paused: AtomicBool::new(start_paused),
            events,
            active: Mutex::new(None),
            next_id: AtomicU64::new(1),
        });
        let accept = tokio::spawn(accept_loop(shared.clone(), listener));
        Ok(Self { shared, accept })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.shared.bound
    }

    pub fn host_public(&self) -> Option<noise::PublicKey> {
        lock(&self.shared.keystore)
            .host_keypair()
            .ok()
            .map(|k| k.public)
    }

    pub fn paired_phone(&self) -> Option<noise::PublicKey> {
        lock(&self.shared.keystore).paired_phone().ok().flatten()
    }

    pub fn is_paused(&self) -> bool {
        self.shared.paused.load(Ordering::SeqCst)
    }

    /// Pause immediately: closes any active session with `error { paused }`.
    pub fn pause(&self) {
        self.shared.paused.store(true, Ordering::SeqCst);
        self.close_active(Some(ErrorCode::Paused));
    }

    pub fn resume(&self) {
        self.shared.paused.store(false, Ordering::SeqCst);
    }

    /// Close the session, keep the pairing.
    pub fn disconnect(&self) {
        self.close_active(None);
    }

    /// Forget the paired phone and close its session.
    pub fn revoke_phone(&self) -> Result<(), crate::keystore::KeystoreError> {
        lock(&self.shared.keystore).revoke_phone()?;
        self.close_active(Some(ErrorCode::Revoked));
        Ok(())
    }

    /// "Reset pairing": new host key, no phone.
    pub fn reset_pairing(&self) -> Result<(), crate::keystore::KeystoreError> {
        lock(&self.shared.keystore).reset_pairing()?;
        lock(&self.shared.pairing).reject();
        self.close_active(Some(ErrorCode::Revoked));
        lock(&self.shared.keystore).host_keypair().map(|_| ())
    }

    /// Open a 120 s pairing window and return the QR content.
    pub fn open_pairing(&self) -> Result<PairingOffer, PairError> {
        if self.is_paused() {
            return Err(PairError::Paused);
        }
        let host = lock(&self.shared.keystore)
            .host_keypair()
            .map_err(|_| PairError::Keystore)?;
        lock(&self.shared.pairing)
            .open(&host.public, self.shared.bound, StdInstant::now())
            .map_err(|_| PairError::Internal)
    }

    pub fn pending_approval(&self) -> Option<pairing::PendingApproval> {
        lock(&self.shared.pairing).pending(StdInstant::now())
    }

    /// The user clicked Approve.
    pub fn approve(&self) -> Result<(), pairing::ApproveError> {
        {
            let ks = lock(&self.shared.keystore);
            lock(&self.shared.pairing).approve(&ks, StdInstant::now())?;
        }
        // Approving replaces and so revokes the previous phone: end any session it still has.
        self.close_active(Some(ErrorCode::Revoked));
        Ok(())
    }

    /// The user clicked Reject.
    pub fn reject(&self) {
        lock(&self.shared.pairing).reject();
    }

    fn close_active(&self, code: Option<ErrorCode>) {
        if let Some(a) = lock(&self.shared.active).as_ref() {
            let _ = a.ctl.send(Control::Close(code));
        }
    }
}

async fn bind_with_fallback(ip: IpAddr, port: u16) -> std::io::Result<(TcpListener, SocketAddr)> {
    let mut last = None;
    for p in 0..PORT_ATTEMPTS {
        let try_port = if port == 0 { 0 } else { port.saturating_add(p) };
        match TcpListener::bind(SocketAddr::new(ip, try_port)).await {
            Ok(l) => {
                let addr = l.local_addr()?;
                return Ok((l, addr));
            }
            Err(e) => last = Some(e),
        }
        if port == 0 {
            break;
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::other("no free port")))
}

async fn accept_loop(sh: Arc<Shared>, listener: TcpListener) {
    loop {
        let Ok((stream, peer)) = listener.accept().await else {
            continue;
        };
        let _ = stream.set_nodelay(true);
        // Over-limit, blocked and non-LAN sources are dropped without any response.
        let Ok(guard) = sh.limiter.admit(peer.ip(), StdInstant::now()) else {
            continue;
        };
        tokio::spawn(handle_conn(sh.clone(), stream, peer, guard));
    }
}

enum Handshaken {
    Session(Ws2, Transport),
    Pairing(Ws2, PairingOutcome),
}

async fn handle_conn(sh: Arc<Shared>, stream: TcpStream, peer: SocketAddr, guard: ConnGuard) {
    let result = timeout(sh.cfg.handshake_timeout, handshake(&sh, stream)).await;
    drop(guard);
    match result {
        Ok(Some(Handshaken::Session(ws, tr))) => run_session(sh, ws, tr).await,
        Ok(Some(Handshaken::Pairing(ws, outcome))) => finish_pairing(sh, ws, outcome).await,
        _ => sh.limiter.record_failure(peer.ip(), StdInstant::now()),
    }
}

/// Peek the HTTP request head without consuming it, so the WebSocket library can read it.
async fn peek_head(stream: &TcpStream) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; MAX_HTTP_HEAD];
    loop {
        let n = stream.peek(&mut buf).await.ok()?;
        if n == 0 {
            return None;
        }
        if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") || n == MAX_HTTP_HEAD {
            buf.truncate(n);
            return Some(buf);
        }
        sleep(Duration::from_millis(10)).await;
    }
}

fn is_ws_request(head: &[u8], bound: SocketAddr) -> bool {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    if !matches!(req.parse(head), Ok(httparse::Status::Complete(_))) {
        return false;
    }
    if req.method != Some("GET") || req.path != Some("/ws") {
        return false;
    }
    let mut origins = req
        .headers
        .iter()
        .filter(|h| h.name.eq_ignore_ascii_case("origin"));
    let origin = match (origins.next(), origins.next()) {
        (None, _) => None,
        (Some(h), None) => match std::str::from_utf8(h.value) {
            Ok(v) => Some(v),
            Err(_) => return false,
        },
        (Some(_), Some(_)) => return false,
    };
    if !net::origin_allowed(origin, bound) {
        return false;
    }
    req.headers.iter().any(|h| {
        h.name.eq_ignore_ascii_case("upgrade")
            && std::str::from_utf8(h.value)
                .is_ok_and(|v| v.trim().eq_ignore_ascii_case("websocket"))
    })
}

async fn reject_http(mut stream: TcpStream, head_len: usize) {
    // Consume what we peeked so closing the socket does not reset the connection.
    let mut sink = vec![0u8; head_len];
    let _ = stream.read_exact(&mut sink).await;
    let _ = stream.write_all(NOT_FOUND).await;
    let _ = stream.shutdown().await;
}

async fn next_binary(ws: &mut Ws2) -> Option<Vec<u8>> {
    loop {
        match ws.next().await? {
            Ok(Ws::Binary(b)) => return Some(b.to_vec()),
            Ok(Ws::Ping(_) | Ws::Pong(_)) => continue,
            _ => return None, // text frames, close frames and errors all end the connection
        }
    }
}

async fn send_binary(ws: &mut Ws2, data: Vec<u8>) -> bool {
    ws.send(Ws::Binary(data.into())).await.is_ok()
}

async fn handshake(sh: &Arc<Shared>, stream: TcpStream) -> Option<Handshaken> {
    let head = peek_head(&stream).await?;
    if !is_ws_request(&head, sh.bound) {
        reject_http(stream, head.len()).await;
        return None;
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_TRANSPORT_FRAME))
        .max_frame_size(Some(MAX_TRANSPORT_FRAME));
    let mut ws = accept_async_with_config(stream, Some(config)).await.ok()?;
    let first = next_binary(&mut ws).await?;
    let (kind, body) = noise::split_handshake_frame(&first).ok()?;
    match kind {
        HandshakeKind::Session => {
            let host = lock(&sh.keystore).host_keypair().ok()?;
            let phone = lock(&sh.keystore).paired_phone().ok().flatten()?;
            let responder = SessionResponder::new(&host.private, &phone).ok()?;
            let (reply, transport) = responder.respond(body).ok()?;
            let frame = noise::encode_handshake_frame(HandshakeKind::Session, &reply);
            send_binary(&mut ws, frame).await.then_some(())?;
            Some(Handshaken::Session(ws, transport))
        }
        HandshakeKind::Pairing => {
            if sh.paused.load(Ordering::SeqCst) {
                return None;
            }
            let host = lock(&sh.keystore).host_keypair().ok()?;
            let mut responder = lock(&sh.pairing)
                .begin_attempt(&host.private, StdInstant::now())
                .ok()?;
            let _ = sh.events.send(ServerEvent::PairingAttempt);
            let outcome = async {
                responder.read_msg1(body).ok()?;
                let m2 = responder.write_msg2().ok()?;
                let frame = noise::encode_handshake_frame(HandshakeKind::Pairing, &m2);
                send_binary(&mut ws, frame).await.then_some(())?;
                let third = next_binary(&mut ws).await?;
                let (k, b3) = noise::split_handshake_frame(&third).ok()?;
                (k == HandshakeKind::Pairing).then_some(())?;
                responder.read_msg3(b3).ok()?;
                responder.finish().ok()
            }
            .await;
            match outcome {
                Some(o) => Some(Handshaken::Pairing(ws, o)),
                None => {
                    lock(&sh.pairing).attempt_failed(StdInstant::now());
                    None
                }
            }
        }
    }
}

async fn finish_pairing(sh: Arc<Shared>, mut ws: Ws2, outcome: PairingOutcome) {
    let PairingOutcome {
        phone_public,
        code,
        mut transport,
    } = outcome;
    let rx = lock(&sh.pairing).attempt_succeeded(phone_public, code.clone(), StdInstant::now());
    let Ok(rx) = rx else {
        let _ = ws.close(None).await;
        return;
    };
    let _ = sh.events.send(ServerEvent::ApprovalRequested { code });
    let decision = match timeout(pairing::APPROVAL_TIMEOUT + Duration::from_secs(2), rx).await {
        Ok(Ok(d)) => d,
        _ => {
            lock(&sh.pairing).tick(StdInstant::now() + pairing::APPROVAL_TIMEOUT);
            Decision::TimedOut
        }
    };
    let approved = decision == Decision::Approved;
    if approved {
        let paired = Message::Paired {
            host_name: sh.cfg.host_name.clone(),
        };
        if let Ok(frame) = transport.encrypt(&paired.encode()) {
            let _ = send_binary(&mut ws, frame).await;
        }
    }
    let _ = sh.events.send(ServerEvent::PairingFinished { approved });
    let _ = ws.close(None).await;
}

async fn send_msg(ws: &mut Ws2, tr: &mut Transport, msg: &Message) -> bool {
    match tr.encrypt(&msg.encode()) {
        Ok(frame) => send_binary(ws, frame).await,
        Err(_) => false,
    }
}

async fn run_session(sh: Arc<Shared>, mut ws: Ws2, mut tr: Transport) {
    let id = sh.next_id.fetch_add(1, Ordering::SeqCst);
    let (ctl_tx, mut ctl_rx) = mpsc::unbounded_channel();
    // A new authenticated session replaces the old one.
    if let Some(old) = lock(&sh.active).replace(ActiveSession { id, ctl: ctl_tx }) {
        let _ = old.ctl.send(Control::Close(Some(ErrorCode::Replaced)));
    }
    let _ = sh.events.send(ServerEvent::PhoneConnected);
    let end_code = session_loop(&sh, &mut ws, &mut tr, &mut ctl_rx).await;
    if let Some(code) = end_code {
        let _ = send_msg(&mut ws, &mut tr, &Message::Error { code }).await;
    }
    let _ = ws.close(None).await;
    // Always release held buttons and keys when a session ends.
    sh.sink.release_all();
    let mut active = lock(&sh.active);
    if active.as_ref().is_some_and(|a| a.id == id) {
        *active = None;
        let _ = sh.events.send(ServerEvent::PhoneDisconnected);
    }
}

/// Returns the error code to send before closing, if any.
async fn session_loop(
    sh: &Arc<Shared>,
    ws: &mut Ws2,
    tr: &mut Transport,
    ctl: &mut mpsc::UnboundedReceiver<Control>,
) -> Option<ErrorCode> {
    if sh.paused.load(Ordering::SeqCst) {
        return Some(ErrorCode::Paused);
    }
    let status = Message::Status {
        platform: sh.cfg.platform,
        profile: "generic".into(),
        profiles: vec!["generic".into()],
        paused: false,
    };
    if !send_msg(ws, tr, &status).await {
        return None;
    }
    let mut rate = RateWindow::new(StdInstant::now());
    let mut last_rx = Instant::now();
    let mut last_input = Instant::now();
    loop {
        tokio::select! {
            biased;
            c = ctl.recv() => {
                return match c {
                    Some(Control::Close(code)) => code,
                    None => None,
                };
            }
            _ = sleep_until(last_rx + sh.cfg.silence_timeout) => return None,
            _ = sleep_until(last_input + sh.cfg.idle_timeout) => return Some(ErrorCode::Idle),
            frame = ws.next() => {
                let data = match frame {
                    Some(Ok(Ws::Binary(b))) => b,
                    Some(Ok(Ws::Ping(_) | Ws::Pong(_))) => continue,
                    _ => return None, // text frame, close frame or error
                };
                last_rx = Instant::now();
                if !rate.allow(StdInstant::now()) {
                    return Some(ErrorCode::RateLimited);
                }
                let Ok(plain) = tr.decrypt(&data) else { return None };
                let Ok(msg) = Message::decode(&plain) else { return None };
                if !msg.is_phone_to_host() {
                    return None;
                }
                match &msg {
                    Message::Ping { t } => {
                        if !send_msg(ws, tr, &Message::Pong { t: *t }).await {
                            return None;
                        }
                    }
                    Message::Pong { .. } => {}
                    _ => {
                        last_input = Instant::now();
                        sh.sink.handle(&msg);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keystore::MemoryStore;
    use crate::noise::{
        PAIRING_PROTOCOL, PublicKey, SESSION_PROTOCOL, StaticKeypair, encode_handshake_frame,
        generate_static_keypair, pairing_code, split_handshake_frame,
    };
    use crate::protocol::{ButtonState, KeyName, MouseButton};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use snow::Builder;
    use std::collections::VecDeque;
    use tokio::net::TcpStream as Tcp;
    use tokio_tungstenite::{MaybeTlsStream, connect_async};

    type Client = WebSocketStream<MaybeTlsStream<Tcp>>;

    #[derive(Default)]
    struct Recorder {
        msgs: Mutex<Vec<Message>>,
        releases: AtomicU64,
    }

    impl CommandSink for Recorder {
        fn handle(&self, msg: &Message) {
            lock(&self.msgs).push(msg.clone());
        }
        fn release_all(&self) {
            self.releases.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Harness {
        server: Server,
        addr: SocketAddr,
        host_pub: PublicKey,
        phone: StaticKeypair,
        rec: Arc<Recorder>,
        events: mpsc::UnboundedReceiver<ServerEvent>,
    }

    async fn harness_with(
        paired: bool,
        paused: bool,
        tweak: impl FnOnce(&mut ServerConfig),
    ) -> Harness {
        let ks = Keystore::new(Box::new(MemoryStore::default()));
        let host = ks.host_keypair().unwrap();
        let phone = generate_static_keypair().unwrap();
        if paired {
            ks.set_paired_phone(&phone.public).unwrap();
        }
        let mut cfg = ServerConfig::new(
            "127.0.0.1".parse().unwrap(),
            "Test Mac".into(),
            Platform::Macos,
        );
        cfg.port = 0;
        tweak(&mut cfg);
        let rec = Arc::new(Recorder::default());
        let (tx, events) = mpsc::unbounded_channel();
        let server = Server::start(cfg, ks, rec.clone(), tx, paused)
            .await
            .unwrap();
        Harness {
            addr: server.local_addr(),
            server,
            host_pub: host.public,
            phone,
            rec,
            events,
        }
    }

    async fn harness() -> Harness {
        harness_with(true, false, |_| {}).await
    }

    async fn connect(addr: SocketAddr) -> Client {
        connect_async(format!("ws://{addr}/ws")).await.unwrap().0
    }

    async fn next_bin(ws: &mut Client) -> Option<Vec<u8>> {
        loop {
            match timeout(Duration::from_secs(3), ws.next())
                .await
                .expect("timed out waiting")
            {
                Some(Ok(Ws::Binary(b))) => return Some(b.to_vec()),
                Some(Ok(Ws::Ping(_) | Ws::Pong(_))) => continue,
                _ => return None,
            }
        }
    }

    struct Phone {
        ws: Client,
        tr: Transport,
        backlog: VecDeque<Message>,
    }

    impl Phone {
        async fn send(&mut self, msg: &Message) {
            let frame = self.tr.encrypt(&msg.encode()).unwrap();
            self.ws.send(Ws::Binary(frame.into())).await.unwrap();
        }
        async fn recv(&mut self) -> Option<Message> {
            if let Some(m) = self.backlog.pop_front() {
                return Some(m);
            }
            let frame = next_bin(&mut self.ws).await?;
            let plain = self.tr.decrypt(&frame).ok()?;
            Some(Message::decode(&plain).unwrap())
        }
        async fn expect_closed(&mut self) -> Vec<Message> {
            let mut seen = Vec::new();
            while let Some(m) = self.recv().await {
                seen.push(m);
            }
            seen
        }
    }

    async fn try_session(
        addr: SocketAddr,
        phone: &StaticKeypair,
        host_pub: &PublicKey,
    ) -> Option<Phone> {
        let mut ws = connect(addr).await;
        let mut init = Builder::new(SESSION_PROTOCOL.parse().unwrap())
            .local_private_key(&*phone.private)
            .unwrap()
            .remote_public_key(host_pub)
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut buf = [0u8; 512];
        let mut p = [0u8; 512];
        let n = init.write_message(&[], &mut buf).unwrap();
        let frame = encode_handshake_frame(HandshakeKind::Session, &buf[..n]);
        ws.send(Ws::Binary(frame.into())).await.ok()?;
        let reply = next_bin(&mut ws).await?;
        let (_, body) = split_handshake_frame(&reply).ok()?;
        init.read_message(body, &mut p).ok()?;
        let tr = Transport::new(init.into_transport_mode().unwrap());
        Some(Phone {
            ws,
            tr,
            backlog: VecDeque::new(),
        })
    }

    /// Session where the first message is the host's `status`.
    async fn session(h: &Harness) -> Phone {
        let mut p = try_session(h.addr, &h.phone, &h.host_pub)
            .await
            .expect("handshake failed");
        assert!(matches!(p.recv().await, Some(Message::Status { .. })));
        p
    }

    async fn eventually(what: &str, f: impl Fn() -> bool) {
        for _ in 0..200 {
            if f() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("condition never became true: {what}");
    }

    async fn raw_request(addr: SocketAddr, request: &str) -> Vec<u8> {
        let mut s = Tcp::connect(addr).await.unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut out = Vec::new();
        timeout(Duration::from_secs(3), s.read_to_end(&mut out))
            .await
            .unwrap()
            .ok();
        out
    }

    #[tokio::test]
    async fn refuses_to_bind_unsafe_addresses() {
        for ip in ["0.0.0.0", "::", "8.8.8.8", "100.64.0.1"] {
            let cfg = ServerConfig::new(ip.parse().unwrap(), "x".into(), Platform::Macos);
            let ks = Keystore::new(Box::new(MemoryStore::default()));
            let (tx, _rx) = mpsc::unbounded_channel();
            let r = Server::start(cfg, ks, Arc::new(Recorder::default()), tx, false).await;
            assert!(matches!(r, Err(StartError::UnsafeBind)), "{ip}");
        }
    }

    #[test]
    fn idle_timeout_cannot_go_below_five_minutes() {
        let mut cfg = ServerConfig::new("127.0.0.1".parse().unwrap(), "x".into(), Platform::Macos);
        cfg.set_idle_timeout(Duration::from_secs(1));
        assert_eq!(cfg.idle_timeout, MIN_IDLE_TIMEOUT);
        cfg.set_idle_timeout(Duration::from_secs(3600));
        assert_eq!(cfg.idle_timeout, Duration::from_secs(3600));
    }

    #[tokio::test]
    async fn every_path_and_method_except_get_ws_returns_empty_404() {
        let h = harness().await;
        let origin_bad = "GET /ws HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nOrigin: http://evil.example\r\n\r\n";
        for req in [
            "GET / HTTP/1.1\r\nHost: x\r\n\r\n",
            "POST /ws HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n",
            "GET /ws HTTP/1.1\r\nHost: x\r\n\r\n",
            "GET /ws/ HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
            origin_bad,
        ] {
            let resp = raw_request(h.addr, req).await;
            let text = String::from_utf8_lossy(&resp);
            assert!(text.starts_with("HTTP/1.1 404"), "{req:?} -> {text:?}");
            assert!(text.contains("Content-Length: 0"));
            assert!(text.ends_with("\r\n\r\n"), "body must be empty: {text:?}");
            sleep(Duration::from_millis(30)).await;
        }
    }

    #[tokio::test]
    async fn session_forwards_commands_and_releases_on_disconnect() {
        let mut h = harness().await;
        let mut p = session(&h).await;
        assert_eq!(h.events.recv().await, Some(ServerEvent::PhoneConnected));
        p.send(&Message::Ping { t: 42 }).await;
        assert_eq!(p.recv().await, Some(Message::Pong { t: 42 }));
        p.send(&Message::Button {
            button: MouseButton::Left,
            state: ButtonState::Down,
        })
        .await;
        p.send(&Message::Key {
            key: KeyName::Enter,
        })
        .await;
        eventually("commands recorded", || lock(&h.rec.msgs).len() == 2).await;
        assert_eq!(h.rec.releases.load(Ordering::SeqCst), 0);
        drop(p);
        eventually("held input released", || {
            h.rec.releases.load(Ordering::SeqCst) == 1
        })
        .await;
        assert_eq!(h.events.recv().await, Some(ServerEvent::PhoneDisconnected));
    }

    #[tokio::test]
    async fn unknown_phone_key_is_refused() {
        let h = harness().await;
        let stranger = generate_static_keypair().unwrap();
        assert!(try_session(h.addr, &stranger, &h.host_pub).await.is_none());
    }

    #[tokio::test]
    async fn session_is_refused_when_no_phone_is_paired() {
        let h = harness_with(false, false, |_| {}).await;
        assert!(try_session(h.addr, &h.phone, &h.host_pub).await.is_none());
    }

    #[tokio::test]
    async fn text_frame_closes_the_session() {
        let h = harness().await;
        let mut p = session(&h).await;
        p.ws.send(Ws::Text("hello".into())).await.unwrap();
        assert!(p.expect_closed().await.is_empty());
    }

    #[tokio::test]
    async fn silent_handshake_times_out() {
        let h = harness_with(true, false, |c| {
            c.handshake_timeout = Duration::from_millis(300)
        })
        .await;
        let mut ws = connect(h.addr).await;
        assert!(next_bin(&mut ws).await.is_none());
    }

    #[tokio::test]
    async fn tampered_garbled_and_oversized_frames_close_the_session() {
        let h = harness().await;
        let mut p = session(&h).await;
        let mut frame = p.tr.encrypt(&Message::Ping { t: 1 }.encode()).unwrap();
        frame[0] ^= 1;
        p.ws.send(Ws::Binary(frame.into())).await.unwrap();
        assert!(p.expect_closed().await.is_empty());
        sleep(Duration::from_millis(30)).await;

        let mut p = session(&h).await;
        let frame = p.tr.encrypt(b"not json").unwrap();
        p.ws.send(Ws::Binary(frame.into())).await.unwrap();
        assert!(p.expect_closed().await.is_empty());
        sleep(Duration::from_millis(30)).await;

        let mut p = session(&h).await;
        p.ws.send(Ws::Binary(vec![0u8; MAX_TRANSPORT_FRAME + 1].into()))
            .await
            .ok();
        assert!(p.expect_closed().await.is_empty());
    }

    #[tokio::test]
    async fn host_to_phone_messages_from_the_phone_close_the_session() {
        let h = harness().await;
        let mut p = session(&h).await;
        p.send(&Message::Error {
            code: ErrorCode::Idle,
        })
        .await;
        assert!(p.expect_closed().await.is_empty());
        assert!(lock(&h.rec.msgs).is_empty());
    }

    #[tokio::test]
    async fn second_session_replaces_the_first() {
        let h = harness().await;
        let mut first = session(&h).await;
        sleep(Duration::from_millis(30)).await;
        let mut second = session(&h).await;
        let seen = first.expect_closed().await;
        assert_eq!(
            seen,
            vec![Message::Error {
                code: ErrorCode::Replaced
            }]
        );
        second.send(&Message::Ping { t: 7 }).await;
        assert_eq!(second.recv().await, Some(Message::Pong { t: 7 }));
    }

    #[tokio::test]
    async fn silence_for_too_long_drops_the_session() {
        let h = harness_with(true, false, |c| {
            c.silence_timeout = Duration::from_millis(400)
        })
        .await;
        let mut p = session(&h).await;
        assert!(p.expect_closed().await.is_empty());
        eventually("released", || h.rec.releases.load(Ordering::SeqCst) == 1).await;
    }

    #[tokio::test]
    async fn no_input_for_too_long_sends_idle_and_closes() {
        let h = harness_with(true, false, |c| c.idle_timeout = Duration::from_millis(400)).await;
        let mut p = session(&h).await;
        for t in 0..8 {
            let frame = p.tr.encrypt(&Message::Ping { t }.encode()).unwrap();
            if p.ws.send(Ws::Binary(frame.into())).await.is_err() {
                break; // the host already closed the session
            }
            sleep(Duration::from_millis(100)).await;
        }
        let seen = p.expect_closed().await;
        assert!(
            seen.contains(&Message::Error {
                code: ErrorCode::Idle
            }),
            "{seen:?}"
        );
    }

    #[tokio::test]
    async fn more_than_250_messages_per_second_closes_the_session() {
        let h = harness().await;
        let mut p = session(&h).await;
        for t in 0..300 {
            if p.ws
                .send(Ws::Binary(
                    p.tr.encrypt(&Message::Ping { t }.encode()).unwrap().into(),
                ))
                .await
                .is_err()
            {
                break;
            }
        }
        let seen = p.expect_closed().await;
        assert!(
            seen.contains(&Message::Error {
                code: ErrorCode::RateLimited
            }),
            "{} msgs",
            seen.len()
        );
    }

    #[tokio::test]
    async fn only_one_pre_handshake_connection_per_ip() {
        let h = harness().await;
        let _first = Tcp::connect(h.addr).await.unwrap();
        sleep(Duration::from_millis(50)).await;
        let mut second = Tcp::connect(h.addr).await.unwrap();
        let mut buf = [0u8; 16];
        let n = timeout(Duration::from_secs(2), second.read(&mut buf))
            .await
            .unwrap()
            .unwrap_or(0);
        assert_eq!(n, 0, "second connection must be dropped without a reply");
    }

    #[tokio::test]
    async fn five_failed_handshakes_block_the_source_for_a_minute() {
        let h = harness().await;
        for _ in 0..5 {
            let _ = raw_request(h.addr, "GET /nope HTTP/1.1\r\nHost: x\r\n\r\n").await;
            sleep(Duration::from_millis(40)).await;
        }
        let mut s = Tcp::connect(h.addr).await.unwrap();
        let mut buf = [0u8; 16];
        let n = timeout(Duration::from_secs(2), s.read(&mut buf))
            .await
            .unwrap()
            .unwrap_or(0);
        assert_eq!(n, 0, "blocked source gets nothing");
        // and a legitimate phone from the same address is refused too
        assert!(try_session_blocked(h.addr).await);
    }

    async fn try_session_blocked(addr: SocketAddr) -> bool {
        match connect_async(format!("ws://{addr}/ws")).await {
            Err(_) => true,
            Ok((mut ws, _)) => next_bin(&mut ws).await.is_none(),
        }
    }

    #[tokio::test]
    async fn paused_server_tells_a_known_phone_and_closes() {
        let h = harness_with(true, true, |_| {}).await;
        let mut p = try_session(h.addr, &h.phone, &h.host_pub).await.unwrap();
        let seen = p.expect_closed().await;
        assert_eq!(
            seen,
            vec![Message::Error {
                code: ErrorCode::Paused
            }]
        );
        assert!(h.server.open_pairing().is_err(), "no pairing while paused");
    }

    #[tokio::test]
    async fn pause_closes_the_active_session() {
        let h = harness().await;
        let mut p = session(&h).await;
        h.server.pause();
        assert_eq!(
            p.expect_closed().await,
            vec![Message::Error {
                code: ErrorCode::Paused
            }]
        );
        h.server.resume();
    }

    #[tokio::test]
    async fn disconnect_keeps_pairing_and_revoke_removes_it() {
        let h = harness().await;
        let mut p = session(&h).await;
        h.server.disconnect();
        assert!(p.expect_closed().await.is_empty());
        assert!(h.server.paired_phone().is_some());
        sleep(Duration::from_millis(30)).await;
        let mut p = session(&h).await;
        h.server.revoke_phone().unwrap();
        assert_eq!(
            p.expect_closed().await,
            vec![Message::Error {
                code: ErrorCode::Revoked
            }]
        );
        assert!(h.server.paired_phone().is_none());
        sleep(Duration::from_millis(30)).await;
        assert!(try_session(h.addr, &h.phone, &h.host_pub).await.is_none());
    }

    struct PairingPhone {
        ws: Client,
        init: snow::HandshakeState,
        code: Option<String>,
        host_key_ok: bool,
    }

    fn token_and_hk(uri: &str) -> ([u8; 32], Vec<u8>) {
        let get = |k: &str| -> Vec<u8> {
            let v = uri
                .split(['?', '&'])
                .find_map(|kv| kv.strip_prefix(k))
                .unwrap();
            URL_SAFE_NO_PAD.decode(v).unwrap()
        };
        (get("pt=").try_into().unwrap(), get("hk="))
    }

    /// Runs the phone side of pairing up to (and including) the third message.
    async fn pair_handshake(
        addr: SocketAddr,
        phone: &StaticKeypair,
        uri: &str,
    ) -> Option<PairingPhone> {
        let (token, hk) = token_and_hk(uri);
        let mut ws = connect(addr).await;
        let mut init = Builder::new(PAIRING_PROTOCOL.parse().unwrap())
            .local_private_key(&*phone.private)
            .unwrap()
            .psk(3, &token)
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut buf = [0u8; 512];
        let mut p = [0u8; 512];
        let n = init.write_message(&[], &mut buf).unwrap();
        ws.send(Ws::Binary(
            encode_handshake_frame(HandshakeKind::Pairing, &buf[..n]).into(),
        ))
        .await
        .ok()?;
        let m2 = next_bin(&mut ws).await?;
        let (_, body) = split_handshake_frame(&m2).ok()?;
        init.read_message(body, &mut p).ok()?;
        let host_key_ok = init.get_remote_static() == Some(hk.as_slice());
        let n = init.write_message(&[], &mut buf).unwrap();
        ws.send(Ws::Binary(
            encode_handshake_frame(HandshakeKind::Pairing, &buf[..n]).into(),
        ))
        .await
        .ok()?;
        let code = Some(pairing_code(init.get_handshake_hash()));
        Some(PairingPhone {
            ws,
            init,
            code,
            host_key_ok,
        })
    }

    async fn next_event(h: &mut Harness) -> ServerEvent {
        timeout(Duration::from_secs(3), h.events.recv())
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn pairing_end_to_end_with_approval() {
        let mut h = harness_with(false, false, |_| {}).await;
        let offer = h.server.open_pairing().unwrap();
        let mut pp = pair_handshake(h.addr, &h.phone, &offer.uri)
            .await
            .expect("pairing handshake");
        assert!(
            pp.host_key_ok,
            "host key from handshake equals hk from the QR"
        );
        assert_eq!(next_event(&mut h).await, ServerEvent::PairingAttempt);
        let ServerEvent::ApprovalRequested { code } = next_event(&mut h).await else {
            panic!("expected approval request");
        };
        assert_eq!(Some(code), pp.code);
        assert!(
            h.server.paired_phone().is_none(),
            "nothing stored before Approve"
        );
        h.server.approve().unwrap();
        let frame = next_bin(&mut pp.ws).await.expect("paired message");
        let mut tr = Transport::new(pp.init.into_transport_mode().unwrap());
        let msg = Message::decode(&tr.decrypt(&frame).unwrap()).unwrap();
        assert_eq!(
            msg,
            Message::Paired {
                host_name: "Test Mac".into()
            }
        );
        assert_eq!(h.server.paired_phone(), Some(h.phone.public));
        assert_eq!(
            next_event(&mut h).await,
            ServerEvent::PairingFinished { approved: true }
        );
        // the QR token is single use
        sleep(Duration::from_millis(50)).await;
        assert!(pair_handshake(h.addr, &h.phone, &offer.uri).await.is_none());
        // and the new key can open a session
        assert!(try_session(h.addr, &h.phone, &h.host_pub).await.is_some());
    }

    #[tokio::test]
    async fn pairing_reject_stores_nothing() {
        let mut h = harness_with(false, false, |_| {}).await;
        let offer = h.server.open_pairing().unwrap();
        let mut pp = pair_handshake(h.addr, &h.phone, &offer.uri).await.unwrap();
        assert_eq!(next_event(&mut h).await, ServerEvent::PairingAttempt);
        assert!(matches!(
            next_event(&mut h).await,
            ServerEvent::ApprovalRequested { .. }
        ));
        h.server.reject();
        assert!(
            next_bin(&mut pp.ws).await.is_none(),
            "no paired message after Reject"
        );
        assert!(h.server.paired_phone().is_none());
        assert_eq!(
            next_event(&mut h).await,
            ServerEvent::PairingFinished { approved: false }
        );
    }

    #[tokio::test]
    async fn approving_a_new_phone_revokes_the_old_one_end_to_end() {
        let mut h = harness().await; // old phone already paired
        let mut old = session(&h).await;
        let offer = h.server.open_pairing().unwrap();
        let new_phone = generate_static_keypair().unwrap();
        let _pp = pair_handshake(h.addr, &new_phone, &offer.uri)
            .await
            .unwrap();
        assert!(matches!(
            h.events.recv().await,
            Some(ServerEvent::PhoneConnected)
        ));
        loop {
            if matches!(
                next_event(&mut h).await,
                ServerEvent::ApprovalRequested { .. }
            ) {
                break;
            }
        }
        h.server.approve().unwrap();
        assert_eq!(h.server.paired_phone(), Some(new_phone.public));
        old.expect_closed().await;
        sleep(Duration::from_millis(50)).await;
        assert!(
            try_session(h.addr, &h.phone, &h.host_pub).await.is_none(),
            "old phone is revoked"
        );
        assert!(try_session(h.addr, &new_phone, &h.host_pub).await.is_some());
    }

    #[tokio::test]
    async fn pairing_window_closes_after_three_failed_attempts() {
        let h = harness_with(false, false, |_| {}).await;
        let offer = h.server.open_pairing().unwrap();
        let real = offer
            .uri
            .split(['?', '&'])
            .find(|kv| kv.starts_with("pt="))
            .unwrap()
            .to_owned();
        let wrong = offer
            .uri
            .replace(&real, &format!("pt={}", URL_SAFE_NO_PAD.encode([9u8; 32])));
        for _ in 0..3 {
            // The phone's writes succeed; the host rejects the third message (wrong token).
            let _ = pair_handshake(h.addr, &h.phone, &wrong).await;
            sleep(Duration::from_millis(80)).await;
        }
        sleep(Duration::from_millis(50)).await;
        assert!(
            pair_handshake(h.addr, &h.phone, &offer.uri).await.is_none(),
            "window must be closed after 3 failed attempts, even for the right token"
        );
    }

    #[tokio::test]
    async fn no_pairing_without_an_open_window() {
        let h = harness_with(false, false, |_| {}).await;
        // A well-formed QR for this host, but the user never clicked "Pair phone".
        let uri = format!(
            "pocketremote://pair?v=1&addr={}&pt={}&hk={}&id=x",
            h.addr,
            URL_SAFE_NO_PAD.encode([1u8; 32]),
            URL_SAFE_NO_PAD.encode(h.host_pub),
        );
        assert!(pair_handshake(h.addr, &h.phone, &uri).await.is_none());
    }
}
