//! TEST-ONLY server for the app's interop test (`npm run test:interop` in app/).
//! Uses an in-memory keystore with throwaway keys, binds 127.0.0.1 and prints JSON lines.
//! Never ship or run this against real keys. It is an example, so `cargo build` skips it.

use pocketremote::keystore::{Keystore, MemoryStore};
use pocketremote::protocol::{Message, Platform};
use pocketremote::server::{CommandSink, Server, ServerConfig, ServerEvent};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

struct PrintSink;

impl CommandSink for PrintSink {
    fn handle(&self, msg: &Message) {
        println!("{}", serde_json::json!({"event": "cmd", "msg": msg}));
    }
    fn release_all(&self) {
        println!("{}", serde_json::json!({"event": "released"}));
    }
}

#[tokio::main]
async fn main() {
    let mut cfg = ServerConfig::new(
        "127.0.0.1".parse().unwrap(),
        "Interop Mac".into(),
        Platform::Macos,
    );
    cfg.port = 0;
    let (tx, mut events) = mpsc::unbounded_channel();
    let server = Server::start(
        cfg,
        Keystore::new(Box::new(MemoryStore::default())),
        Arc::new(PrintSink),
        tx,
        false,
    )
    .await
    .expect("start");
    let offer = server.open_pairing().expect("open pairing");
    println!(
        "{}",
        serde_json::json!({"event": "listening", "addr": server.local_addr().to_string(), "qr": offer.uri})
    );
    tokio::spawn(async move {
        while let Some(ev) = events.recv().await {
            let line = match ev {
                ServerEvent::ApprovalRequested { code } => {
                    serde_json::json!({"event": "approval", "code": code})
                }
                ServerEvent::PhoneConnected => serde_json::json!({"event": "connected"}),
                ServerEvent::PhoneDisconnected => serde_json::json!({"event": "disconnected"}),
                ServerEvent::PairingAttempt => serde_json::json!({"event": "attempt"}),
                ServerEvent::PairingFinished { approved } => {
                    serde_json::json!({"event": "finished", "approved": approved})
                }
            };
            println!("{line}");
        }
    });
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        match line.trim() {
            "approve" => println!(
                "{}",
                serde_json::json!({"event": "approve_result", "ok": server.approve().is_ok()})
            ),
            "reject" => server.reject(),
            "quit" => break,
            _ => {}
        }
    }
}
