//! Starts the real server and executor and forwards what the UI cares about. The UI never touches
//! keys, tokens or typed text: it only sees coarse events and calls the server's sync methods.

use iced::futures::{SinkExt, Stream};
use iced::stream;
use pocketremote::executor::{Executor, Os, Pump};
use pocketremote::keystore::{FileStore, Keystore, SecretStore, open_default_store};
use pocketremote::net::{self, NetworkId, TrustStore};
use pocketremote::paths;
use pocketremote::protocol::Message;
use pocketremote::protocol::Platform;
use pocketremote::server::{CommandSink, Server, ServerConfig, ServerEvent};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use zeroize::Zeroizing;

/// Names and counts only, never values. Set POCKETREMOTE_DEBUG=1 to see which secrets are touched.
fn debug(what: &str) {
    if std::env::var_os("POCKETREMOTE_DEBUG").is_some() {
        eprintln!("[keystore] {what}");
    }
}

/// Reads each secret from the OS key store once and then serves it from memory (zeroized on drop).
/// Without this the server would read the Keychain on every phone connection attempt, and a phone
/// that retries every few seconds would make macOS ask for permission over and over.
struct CachedStore {
    inner: Box<dyn SecretStore>,
    cache: std::sync::Mutex<std::collections::HashMap<String, Option<Zeroizing<Vec<u8>>>>>,
}

impl CachedStore {
    fn new(inner: Box<dyn SecretStore>) -> Self {
        Self {
            inner,
            cache: std::sync::Mutex::default(),
        }
    }
}

type StoreResult<T> = Result<T, pocketremote::keystore::KeystoreError>;

impl SecretStore for CachedStore {
    fn get(&self, name: &str) -> StoreResult<Option<Zeroizing<Vec<u8>>>> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| pocketremote::keystore::KeystoreError::Failed)?;
        if let Some(hit) = cache.get(name) {
            return Ok(hit.as_ref().map(|v| Zeroizing::new((**v).clone())));
        }
        debug(&format!("read {name} from the OS store"));
        let value = self.inner.get(name)?;
        cache.insert(
            name.to_owned(),
            value.as_ref().map(|v| Zeroizing::new((**v).clone())),
        );
        Ok(value)
    }
    fn set(&self, name: &str, value: &[u8]) -> StoreResult<()> {
        debug(&format!("write {name} to the OS store"));
        self.inner.set(name, value)?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| pocketremote::keystore::KeystoreError::Failed)?;
        cache.insert(name.to_owned(), Some(Zeroizing::new(value.to_vec())));
        Ok(())
    }
    fn delete(&self, name: &str) -> StoreResult<()> {
        debug(&format!("delete {name} from the OS store"));
        self.inner.delete(name)?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| pocketremote::keystore::KeystoreError::Failed)?;
        cache.insert(name.to_owned(), None);
        Ok(())
    }
}

/// Hands phone commands to the input worker and wakes the app's main thread to run them. Input must
/// run on the main thread on macOS: the keyboard-layout lookup behind letter keys (Cmd+W, Cmd+R, ...)
/// kills the whole app with SIGTRAP on any other thread.
struct WakingSink {
    exec: Executor,
    wake: mpsc::UnboundedSender<()>,
}

impl CommandSink for WakingSink {
    fn handle(&self, msg: &Message) {
        self.exec.handle(msg);
        let _ = self.wake.send(());
    }
    fn release_all(&self) {
        self.exec.release_all();
        let _ = self.wake.send(());
    }
}

#[derive(Clone)]
pub struct Started {
    pub server: Arc<Server>,
    pub addr: SocketAddr,
    pub host_name: String,
    /// Set when this network has not been allowed yet; the server then starts paused.
    pub untrusted: Option<NetworkId>,
    /// The input worker. Run it with `run_pending()` on the main thread when `Engine::InputReady` arrives.
    pub pump: Arc<std::sync::Mutex<Pump>>,
}

impl std::fmt::Debug for Started {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Started")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub enum Engine {
    Started(Started),
    Event(ServerEvent),
    /// Phone commands are waiting; run `Started::pump` on the main thread.
    InputReady,
    /// macOS Accessibility (or Windows equivalent) not granted yet. Retried every 2 s.
    NoPermission,
    Failed(String),
}

enum StartError {
    NoPermission,
    Failed(String),
}

fn platform() -> Platform {
    match Os::current() {
        Os::Macos => Platform::Macos,
        Os::Windows => Platform::Windows,
        Os::Linux => Platform::Linux,
    }
}

pub fn computer_name() -> String {
    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("/usr/sbin/scutil")
        .args(["--get", "ComputerName"])
        .output()
    {
        let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if !name.is_empty() {
            return name.chars().take(64).collect();
        }
    }
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "This computer".into())
        .chars()
        .take(64)
        .collect()
}

fn backend() -> Result<Box<dyn pocketremote::input::InputBackend>, StartError> {
    // Development only: record input instead of performing it.
    if std::env::var_os("POCKETREMOTE_DRY_RUN").is_some() {
        return Ok(Box::new(pocketremote::input::mock::MockBackend::new()));
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        match pocketremote::input::enigo_backend::EnigoBackend::new() {
            Ok(b) => Ok(Box::new(b)),
            Err(pocketremote::input::InputError::NoPermission) => Err(StartError::NoPermission),
            Err(e) => Err(StartError::Failed(format!("Input backend failed: {e}"))),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(StartError::Failed(
            "No input backend for this system yet.".into(),
        ))
    }
}

type Started3 = (
    Started,
    mpsc::UnboundedReceiver<ServerEvent>,
    mpsc::UnboundedReceiver<()>,
);

async fn start() -> Result<Started3, StartError> {
    debug("starting the server");
    let Some(ip) = net::default_route_ip().filter(|ip| net::ip_allowed(*ip)) else {
        return Err(StartError::Failed(
            "No private network found. Connect to your home Wi-Fi.".into(),
        ));
    };
    let store: Box<dyn SecretStore> = if std::env::var_os("POCKETREMOTE_FILE_KEYS").is_some() {
        let dir = paths::config_dir().map(|d| d.join("keys"));
        dir.and_then(|d| FileStore::new(d).ok())
            .map(|s| Box::new(s) as Box<dyn SecretStore>)
            .ok_or_else(|| StartError::Failed("Could not open the key file store.".into()))?
    } else {
        open_default_store()
            .map_err(|_| StartError::Failed("Could not open the system keychain.".into()))?
    };

    let trust_path = paths::config_dir().map(|d| d.join("trusted-networks.txt"));
    let trust = trust_path
        .as_deref()
        .map_or_else(TrustStore::in_memory, TrustStore::load);
    let network = NetworkId::of(ip);
    let untrusted = match &network {
        Some(id) if trust.is_trusted(id) => None,
        Some(id) => Some(id.clone()),
        // Unknown network: treat as untrusted until the user allows it.
        None => Some(NetworkId::new("unknown network", None)),
    };

    let (exec, pump) = Executor::manual(backend()?, Os::current());
    let (wake_tx, wake_rx) = mpsc::unbounded_channel();
    let executor = Arc::new(WakingSink {
        exec,
        wake: wake_tx,
    });
    let host_name = computer_name();
    let cfg = ServerConfig::new(ip, host_name.clone(), platform());
    let (tx, rx) = mpsc::unbounded_channel();
    let server = Server::start(
        cfg,
        Keystore::new(Box::new(CachedStore::new(store))),
        executor,
        tx,
        untrusted.is_some(),
    )
    .await
    .map_err(|e| StartError::Failed(format!("Could not start the server: {e:?}")))?;
    let addr = server.local_addr();
    Ok((
        Started {
            server: Arc::new(server),
            addr,
            host_name,
            untrusted,
            pump: Arc::new(std::sync::Mutex::new(pump)),
        },
        rx,
        wake_rx,
    ))
}

/// Long-lived worker: start (retrying while a permission is missing), then forward server events.
pub fn worker() -> impl Stream<Item = Engine> {
    stream::channel(64, async |mut out| {
        loop {
            match start().await {
                Ok((started, mut rx, mut wake)) => {
                    let _ = out.send(Engine::Started(started)).await;
                    loop {
                        tokio::select! {
                            ev = rx.recv() => {
                                let Some(ev) = ev else { return };
                                if out.send(Engine::Event(ev)).await.is_err() {
                                    return;
                                }
                            }
                            w = wake.recv() => {
                                if w.is_none() {
                                    return;
                                }
                                // Many commands may have queued: one wake-up runs them all.
                                while wake.try_recv().is_ok() {}
                                if out.send(Engine::InputReady).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(StartError::NoPermission) => {
                    let _ = out.send(Engine::NoPermission).await;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(StartError::Failed(m)) => {
                    let _ = out.send(Engine::Failed(m)).await;
                    std::future::pending::<()>().await;
                }
            }
        }
    })
}

/// Allow phones on this network from now on.
pub fn trust_network(id: NetworkId) {
    let Some(path) = paths::config_dir().map(|d| d.join("trusted-networks.txt")) else {
        return;
    };
    let mut trust = TrustStore::load(&path);
    let _ = trust.trust(id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Counting(Arc<AtomicUsize>, pocketremote::keystore::MemoryStore);
    impl SecretStore for Counting {
        fn get(&self, n: &str) -> StoreResult<Option<Zeroizing<Vec<u8>>>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            self.1.get(n)
        }
        fn set(&self, n: &str, v: &[u8]) -> StoreResult<()> {
            self.1.set(n, v)
        }
        fn delete(&self, n: &str) -> StoreResult<()> {
            self.1.delete(n)
        }
    }

    #[test]
    fn a_phone_command_wakes_the_owner_and_runs_only_when_pumped() {
        use pocketremote::input::mock::MockBackend;
        use pocketremote::protocol::KeyName;
        let mock = MockBackend::new();
        let log = mock.log();
        let (exec, mut pump) = Executor::manual(Box::new(mock), Os::Macos);
        let (wake_tx, mut wake_rx) = mpsc::unbounded_channel();
        let sink = WakingSink {
            exec,
            wake: wake_tx,
        };
        sink.handle(&Message::Key {
            key: KeyName::Enter,
        });
        assert!(wake_rx.try_recv().is_ok(), "the main thread is woken");
        assert!(
            log.lock().unwrap().is_empty(),
            "the command has not run yet"
        );
        pump.run_pending(); // what the app's main thread does on `InputReady`
        assert_eq!(log.lock().unwrap().len(), 1);
    }

    #[test]
    fn reads_the_os_store_only_once_per_secret() {
        let reads = Arc::new(AtomicUsize::new(0));
        let store = CachedStore::new(Box::new(Counting(reads.clone(), Default::default())));
        store.set("k", b"secret").unwrap();
        for _ in 0..10 {
            assert_eq!(
                store.get("k").unwrap().as_deref().map(Vec::as_slice),
                Some(&b"secret"[..])
            );
        }
        assert!(store.get("missing").unwrap().is_none());
        assert!(store.get("missing").unwrap().is_none());
        assert_eq!(
            reads.load(Ordering::SeqCst),
            1,
            "only the first read of a missing key reaches the OS store"
        );
        store.delete("k").unwrap();
        assert!(store.get("k").unwrap().is_none());
    }
}
