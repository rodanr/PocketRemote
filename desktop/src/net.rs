//! Network exposure rules (PLAN.md §3.6, §3.9): source-IP allowlist, Origin check,
//! pre-handshake connection caps, failed-handshake blocking, message rate cap and
//! network trust. Pure logic with injected clocks, so it is testable without sockets.

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const MAX_PRE_HANDSHAKE_PER_IP: usize = 1;
pub const MAX_PRE_HANDSHAKE_TOTAL: usize = 8;
pub const MAX_FAILED_HANDSHAKES: u32 = 5;
pub const BLOCK_DURATION: Duration = Duration::from_secs(60);
pub const MAX_MESSAGES_PER_SECOND: u32 = 250;

fn v4_in(ip: Ipv4Addr, net: [u8; 4], prefix: u32) -> bool {
    let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
    (u32::from(ip) & mask) == (u32::from(Ipv4Addr::from(net)) & mask)
}

fn v6_in(ip: Ipv6Addr, net: Ipv6Addr, prefix: u32) -> bool {
    let mask = u128::MAX.checked_shl(128 - prefix).unwrap_or(0);
    (u128::from(ip) & mask) == (u128::from(net) & mask)
}

/// Source-IP allowlist: RFC 1918, loopback, IPv6 unique-local and link-local.
/// Everything else is rejected, including `100.64.0.0/10`. IPv4-mapped IPv6 is normalised first.
pub fn ip_allowed(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => {
            v4_in(v4, [10, 0, 0, 0], 8)
                || v4_in(v4, [172, 16, 0, 0], 12)
                || v4_in(v4, [192, 168, 0, 0], 16)
                || v4_in(v4, [127, 0, 0, 0], 8)
        }
        IpAddr::V6(v6) => {
            v6 == Ipv6Addr::LOCALHOST
                || v6_in(v6, Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0), 7)
                || v6_in(v6, Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), 10)
        }
    }
}

/// An `Origin` header, if present, must be exactly `http://<bound-ip>:<port>`.
/// A missing header is allowed (native clients send none).
pub fn origin_allowed(origin: Option<&str>, bound: SocketAddr) -> bool {
    match origin {
        None => true,
        Some(o) => o == format!("http://{bound}"),
    }
}

/// The local address of the interface that holds the default route, found without sending
/// any packet (connecting a UDP socket only selects a route).
pub fn default_route_ip() -> Option<IpAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("192.0.2.1:9").ok()?; // TEST-NET-1, never routed anywhere
    let ip = sock.local_addr().ok()?.ip();
    if ip.is_unspecified() { None } else { Some(ip) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    NotAllowedSource,
    Blocked,
    PerIpLimit,
    TotalLimit,
}

#[derive(Default)]
struct LimiterState {
    active: HashMap<IpAddr, usize>,
    total: usize,
    failures: HashMap<IpAddr, u32>,
    blocked_until: HashMap<IpAddr, Instant>,
}

/// Pre-handshake caps and failed-handshake blocking. Cloning shares the state.
#[derive(Clone, Default)]
pub struct Limiter(Arc<Mutex<LimiterState>>);

/// Held while a connection is in its pre-handshake phase; releases its slot on drop.
pub struct ConnGuard {
    limiter: Limiter,
    ip: IpAddr,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        let mut st = self.limiter.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = st.active.get_mut(&self.ip) {
            *n -= 1;
            if *n == 0 {
                st.active.remove(&self.ip);
            }
        }
        st.total = st.total.saturating_sub(1);
    }
}

impl Limiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit a new connection: allowed source, not blocked, within the caps.
    pub fn admit(&self, ip: IpAddr, now: Instant) -> Result<ConnGuard, Reject> {
        let ip = ip.to_canonical();
        if !ip_allowed(ip) {
            return Err(Reject::NotAllowedSource);
        }
        let mut st = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(until) = st.blocked_until.get(&ip).copied() {
            if now < until {
                return Err(Reject::Blocked);
            }
            st.blocked_until.remove(&ip);
            st.failures.remove(&ip);
        }
        if st.total >= MAX_PRE_HANDSHAKE_TOTAL {
            return Err(Reject::TotalLimit);
        }
        if st.active.get(&ip).copied().unwrap_or(0) >= MAX_PRE_HANDSHAKE_PER_IP {
            return Err(Reject::PerIpLimit);
        }
        *st.active.entry(ip).or_insert(0) += 1;
        st.total += 1;
        drop(st);
        Ok(ConnGuard {
            limiter: self.clone(),
            ip,
        })
    }

    /// A handshake from this IP failed. Five failures block it for 60 s.
    pub fn record_failure(&self, ip: IpAddr, now: Instant) {
        let ip = ip.to_canonical();
        let mut st = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let n = st.failures.entry(ip).or_insert(0);
        *n += 1;
        if *n >= MAX_FAILED_HANDSHAKES {
            st.failures.remove(&ip);
            st.blocked_until.insert(ip, now + BLOCK_DURATION);
        }
    }

    pub fn is_blocked(&self, ip: IpAddr, now: Instant) -> bool {
        let st = self.0.lock().unwrap_or_else(|e| e.into_inner());
        st.blocked_until
            .get(&ip.to_canonical())
            .is_some_and(|until| now < *until)
    }
}

/// Fixed one-second window message counter: more than 250 messages in a second closes the session.
pub struct RateWindow {
    start: Instant,
    count: u32,
    limit: u32,
}

impl RateWindow {
    pub fn new(now: Instant) -> Self {
        Self::with_limit(now, MAX_MESSAGES_PER_SECOND)
    }

    /// For tests that must trigger the cap on slow machines. Production uses `new`.
    pub fn with_limit(now: Instant, limit: u32) -> Self {
        Self {
            start: now,
            count: 0,
            limit,
        }
    }

    /// Returns false when the message exceeds the cap.
    pub fn allow(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.start) >= Duration::from_secs(1) {
            self.start = now;
            self.count = 0;
        }
        self.count += 1;
        self.count <= self.limit
    }
}

/// Identifies a network by subnet and, if obtainable, gateway IP.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkId(String);

impl NetworkId {
    pub fn new(subnet: &str, gateway: Option<IpAddr>) -> Self {
        match gateway {
            Some(g) => Self(format!("{subnet} via {g}")),
            None => Self(format!("{subnet} via unknown")),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The network that `ip` is on, using the interface's real prefix length.
    pub fn of(ip: IpAddr) -> Option<Self> {
        let prefix = if_addrs::get_if_addrs()
            .ok()?
            .into_iter()
            .find_map(|i| match i.addr {
                if_addrs::IfAddr::V4(a) if IpAddr::V4(a.ip) == ip => Some(a.prefixlen),
                if_addrs::IfAddr::V6(a) if IpAddr::V6(a.ip) == ip => Some(a.prefixlen),
                _ => None,
            })?;
        Some(Self::new(&subnet_string(ip, prefix), default_gateway()))
    }
}

fn subnet_string(ip: IpAddr, prefix: u8) -> String {
    match ip {
        IpAddr::V4(v4) => {
            let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
            format!("{}/{prefix}", Ipv4Addr::from(u32::from(v4) & mask))
        }
        IpAddr::V6(v6) => {
            let mask = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
            format!("{}/{prefix}", Ipv6Addr::from(u128::from(v6) & mask))
        }
    }
}

/// Best-effort default gateway IP (no gateway MAC yet). macOS and Linux only.
pub fn default_gateway() -> Option<IpAddr> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("/sbin/route")
            .args(["-n", "get", "default"])
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|l| l.trim().strip_prefix("gateway:"))
            .and_then(|g| g.trim().parse().ok())
    }
    #[cfg(target_os = "linux")]
    {
        let table = std::fs::read_to_string("/proc/net/route").ok()?;
        table.lines().skip(1).find_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.get(1) == Some(&"00000000") {
                let raw = u32::from_str_radix(f.get(2)?, 16).ok()?;
                Some(IpAddr::V4(Ipv4Addr::from(raw.swap_bytes())))
            } else {
                None
            }
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// Networks the user has said phones may use. A new network starts the server paused.
pub struct TrustStore {
    path: Option<PathBuf>,
    trusted: HashSet<NetworkId>,
}

impl TrustStore {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            trusted: HashSet::new(),
        }
    }

    pub fn load(path: &Path) -> Self {
        let trusted = std::fs::read_to_string(path)
            .map(|s| {
                s.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| NetworkId(l.to_owned()))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            path: Some(path.to_owned()),
            trusted,
        }
    }

    pub fn is_trusted(&self, id: &NetworkId) -> bool {
        self.trusted.contains(id)
    }

    pub fn trust(&mut self, id: NetworkId) -> std::io::Result<()> {
        self.trusted.insert(id);
        self.save()
    }

    pub fn forget(&mut self, id: &NetworkId) -> std::io::Result<()> {
        self.trusted.remove(id);
        self.save()
    }

    pub fn list(&self) -> Vec<&NetworkId> {
        let mut v: Vec<_> = self.trusted.iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut lines: Vec<&str> = self.trusted.iter().map(|n| n.as_str()).collect();
        lines.sort_unstable();
        std::fs::write(path, lines.join("\n") + "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn allowlist_accepts_lan_loopback_and_local_v6() {
        for s in [
            "10.0.0.5",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.20",
            "127.0.0.1",
            "127.9.9.9",
            "::1",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "febf::1",
        ] {
            assert!(ip_allowed(ip(s)), "{s} should be allowed");
        }
    }

    #[test]
    fn allowlist_rejects_everything_else() {
        for s in [
            "8.8.8.8",
            "100.64.0.1",
            "100.127.255.255",
            "172.15.255.255",
            "172.32.0.1",
            "192.169.0.1",
            "11.0.0.1",
            "169.254.1.1",
            "0.0.0.0",
            "255.255.255.255",
            "2001:db8::1",
            "2606:4700::1111",
            "fec0::1",
            "::",
            "ff02::1",
        ] {
            assert!(!ip_allowed(ip(s)), "{s} should be rejected");
        }
    }

    #[test]
    fn ipv4_mapped_ipv6_is_normalised_before_checking() {
        assert!(ip_allowed(ip("::ffff:192.168.1.5")));
        assert!(ip_allowed(ip("::ffff:127.0.0.1")));
        assert!(!ip_allowed(ip("::ffff:8.8.8.8")));
        assert!(!ip_allowed(ip("::ffff:100.64.0.1")));
    }

    #[test]
    fn origin_rule() {
        let bound: SocketAddr = "192.168.1.20:9876".parse().unwrap();
        assert!(origin_allowed(None, bound));
        assert!(origin_allowed(Some("http://192.168.1.20:9876"), bound));
        for bad in [
            "",
            "null",
            "http://evil.example",
            "https://192.168.1.20:9876",
            "http://192.168.1.20:9877",
            "http://192.168.1.21:9876",
            "http://192.168.1.20:9876/",
            "http://localhost:9876",
        ] {
            assert!(
                !origin_allowed(Some(bad), bound),
                "{bad:?} should be rejected"
            );
        }
        let v6: SocketAddr = "[fe80::1]:9876".parse().unwrap();
        assert!(origin_allowed(Some("http://[fe80::1]:9876"), v6));
    }

    #[test]
    fn disallowed_source_is_rejected_before_anything_else() {
        let l = Limiter::new();
        assert_eq!(
            l.admit(ip("8.8.8.8"), Instant::now()).err(),
            Some(Reject::NotAllowedSource)
        );
        assert_eq!(
            l.admit(ip("100.64.0.9"), Instant::now()).err(),
            Some(Reject::NotAllowedSource)
        );
    }

    #[test]
    fn one_pre_handshake_connection_per_ip() {
        let l = Limiter::new();
        let now = Instant::now();
        let g = l.admit(ip("192.168.1.5"), now).unwrap();
        assert_eq!(
            l.admit(ip("192.168.1.5"), now).err(),
            Some(Reject::PerIpLimit)
        );
        assert!(l.admit(ip("192.168.1.6"), now).is_ok());
        drop(g);
        assert!(l.admit(ip("192.168.1.5"), now).is_ok());
    }

    #[test]
    fn eight_pre_handshake_connections_in_total() {
        let l = Limiter::new();
        let now = Instant::now();
        let guards: Vec<_> = (1..=8)
            .map(|i| l.admit(ip(&format!("192.168.1.{i}")), now).unwrap())
            .collect();
        assert_eq!(
            l.admit(ip("192.168.1.99"), now).err(),
            Some(Reject::TotalLimit)
        );
        drop(guards);
        assert!(l.admit(ip("192.168.1.99"), now).is_ok());
    }

    #[test]
    fn five_failures_block_for_sixty_seconds() {
        let l = Limiter::new();
        let t0 = Instant::now();
        let a = ip("192.168.1.5");
        for _ in 0..4 {
            l.record_failure(a, t0);
        }
        assert!(!l.is_blocked(a, t0));
        l.record_failure(a, t0);
        assert!(l.is_blocked(a, t0));
        assert_eq!(
            l.admit(a, t0 + Duration::from_secs(59)).err(),
            Some(Reject::Blocked)
        );
        assert!(l.admit(a, t0 + Duration::from_secs(60)).is_ok());
        // other addresses are unaffected
        assert!(l.admit(ip("192.168.1.6"), t0).is_ok());
    }

    #[test]
    fn rate_window_allows_250_per_second() {
        let t0 = Instant::now();
        let mut r = RateWindow::new(t0);
        for _ in 0..250 {
            assert!(r.allow(t0));
        }
        assert!(!r.allow(t0));
        assert!(r.allow(t0 + Duration::from_secs(1)));
    }

    #[test]
    fn subnet_string_masks_host_bits() {
        assert_eq!(subnet_string(ip("192.168.1.77"), 24), "192.168.1.0/24");
        assert_eq!(subnet_string(ip("10.20.30.40"), 8), "10.0.0.0/8");
        assert_eq!(subnet_string(ip("fe80::1234"), 64), "fe80::/64");
    }

    #[test]
    fn trust_store_persists_decisions() {
        let path = std::env::temp_dir().join(format!(
            "pocketremote-trust-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = NetworkId::new("192.168.1.0/24", Some(ip("192.168.1.1")));
        let cafe = NetworkId::new("10.0.0.0/24", Some(ip("10.0.0.1")));
        let mut store = TrustStore::load(&path);
        assert!(!store.is_trusted(&home), "unknown network is not trusted");
        store.trust(home.clone()).unwrap();
        let again = TrustStore::load(&path);
        assert!(again.is_trusted(&home));
        assert!(!again.is_trusted(&cafe));
        let mut again = again;
        again.forget(&home).unwrap();
        assert!(!TrustStore::load(&path).is_trusted(&home));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn default_route_ip_is_a_usable_address_when_present() {
        if let Some(a) = default_route_ip() {
            assert!(!a.is_unspecified());
        }
    }
}
