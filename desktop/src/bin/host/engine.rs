//! Starts the real server and executor and forwards what the UI cares about. The UI never touches
//! keys, tokens or typed text: it only sees coarse events and calls the server's sync methods.

use iced::futures::{SinkExt, Stream};
use iced::stream;
use pocketremote::executor::{Executor, Os};
use pocketremote::keystore::{FileStore, Keystore, SecretStore, open_default_store};
use pocketremote::net::{self, NetworkId, TrustStore};
use pocketremote::paths;
use pocketremote::protocol::Platform;
use pocketremote::server::{Server, ServerConfig, ServerEvent};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Started {
    pub server: Arc<Server>,
    pub addr: SocketAddr,
    pub host_name: String,
    /// Set when this network has not been allowed yet; the server then starts paused.
    pub untrusted: Option<NetworkId>,
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

async fn start() -> Result<(Started, mpsc::UnboundedReceiver<ServerEvent>), StartError> {
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

    let executor = Arc::new(Executor::spawn(backend()?, Os::current()));
    let host_name = computer_name();
    let cfg = ServerConfig::new(ip, host_name.clone(), platform());
    let (tx, rx) = mpsc::unbounded_channel();
    let server = Server::start(cfg, Keystore::new(store), executor, tx, untrusted.is_some())
        .await
        .map_err(|e| StartError::Failed(format!("Could not start the server: {e:?}")))?;
    let addr = server.local_addr();
    Ok((
        Started {
            server: Arc::new(server),
            addr,
            host_name,
            untrusted,
        },
        rx,
    ))
}

/// Long-lived worker: start (retrying while a permission is missing), then forward server events.
pub fn worker() -> impl Stream<Item = Engine> {
    stream::channel(64, async |mut out| {
        loop {
            match start().await {
                Ok((started, mut rx)) => {
                    let _ = out.send(Engine::Started(started)).await;
                    while let Some(ev) = rx.recv().await {
                        if out.send(Engine::Event(ev)).await.is_err() {
                            return;
                        }
                    }
                    return;
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
