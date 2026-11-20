//! PocketRemote host, terminal edition: runs the real server and executor, shows the pairing
//! QR in the terminal and takes simple commands. The tray and Control Panel come next.
//!
//! Flags: `--file-keys` keep keys in a 0600 file instead of the OS keychain (development).
//!        `--print-uri` also print the pairing link as text (development only; it holds the token).
//!        `--dry-run` use a recording backend instead of real input: pairs and connects normally but
//!        never touches the mouse or keyboard (for demos and UI testing).
//! Never prints keys, tokens, typed text or decrypted payloads.
#![forbid(unsafe_code)]

use pocketremote::executor::{Executor, Os};
use pocketremote::keystore::{FileStore, Keystore, open_default_store};
use pocketremote::net::{self, NetworkId, TrustStore};
use pocketremote::paths;
use pocketremote::protocol::Platform;
use pocketremote::server::{Server, ServerConfig, ServerEvent};
use qrcode::QrCode;
use qrcode::render::unicode::Dense1x2;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

const HELP: &str = "commands: pair | approve | reject | disconnect | revoke | pause | resume | status | help | quit";

fn platform() -> Platform {
    match Os::current() {
        Os::Macos => Platform::Macos,
        Os::Windows => Platform::Windows,
        Os::Linux => Platform::Linux,
    }
}

fn computer_name() -> String {
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

/// Black modules on white, whatever the terminal theme, so phone cameras can read it.
fn print_qr(data: &str) {
    let Ok(code) = QrCode::new(data.as_bytes()) else {
        println!("could not build a QR code");
        return;
    };
    let text = code
        .render::<Dense1x2>()
        .quiet_zone(true)
        .dark_color(Dense1x2::Dark)
        .light_color(Dense1x2::Light)
        .build();
    for line in text.lines() {
        // black text on a white background: dark modules print as black blocks
        println!("  \x1b[30;47m{line}\x1b[0m");
    }
}

fn grouped(code: &str) -> String {
    let (a, b) = code.split_at(code.len().min(3));
    format!("{a} {b}")
}

async fn ask_yes_no(lines: &mut tokio::io::Lines<BufReader<tokio::io::Stdin>>, q: &str) -> bool {
    println!("{q} [y/N]");
    matches!(lines.next_line().await, Ok(Some(l)) if l.trim().eq_ignore_ascii_case("y"))
}

/// The input backend: a recording mock for `--dry-run`, otherwise the real one for this OS.
fn make_backend(dry_run: bool) -> Box<dyn pocketremote::input::InputBackend> {
    if dry_run {
        println!("DRY RUN: input is recorded, not performed.");
        return Box::new(pocketremote::input::mock::MockBackend::new());
    }
    real_backend()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn real_backend() -> Box<dyn pocketremote::input::InputBackend> {
    match pocketremote::input::enigo_backend::EnigoBackend::new() {
        Ok(b) => Box::new(b),
        Err(pocketremote::input::InputError::NoPermission) => {
            eprintln!(
                "Input permission missing. macOS: System Settings > Privacy & Security > Accessibility, enable this terminal, then restart it."
            );
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("Input backend failed: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn real_backend() -> Box<dyn pocketremote::input::InputBackend> {
    eprintln!(
        "No input backend for this OS yet (Linux comes in a later phase). Use --dry-run to try the rest."
    );
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    let file_keys = std::env::args().any(|a| a == "--file-keys");
    let print_uri = std::env::args().any(|a| a == "--print-uri");
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    let Some(ip) = net::default_route_ip().filter(|ip| net::ip_allowed(*ip)) else {
        eprintln!("No private LAN address found. Connect to your home Wi-Fi and try again.");
        std::process::exit(1);
    };

    let store = if file_keys {
        let dir = paths::config_dir().map(|d| d.join("keys"));
        dir.and_then(|d| FileStore::new(d).ok())
            .map(|s| Box::new(s) as Box<dyn pocketremote::keystore::SecretStore>)
    } else {
        open_default_store().ok()
    };
    let Some(store) = store else {
        eprintln!("No key store available.");
        std::process::exit(1);
    };

    // Network trust (PLAN.md §3.6): a network not seen before starts the server paused.
    let trust_path = paths::config_dir().map(|d| d.join("trusted-networks.txt"));
    let mut trust = trust_path
        .as_deref()
        .map_or_else(TrustStore::in_memory, TrustStore::load);
    let mut start_paused = false;
    match NetworkId::of(ip) {
        Some(id) if trust.is_trusted(&id) => println!("Network: {} (trusted)", id.as_str()),
        Some(id) => {
            if ask_yes_no(
                &mut lines,
                &format!("New network {}. Allow phones on it?", id.as_str()),
            )
            .await
            {
                let _ = trust.trust(id);
            } else {
                start_paused = true;
            }
        }
        None => {
            println!(
                "Could not identify this network, so the server starts paused. Type 'resume' to allow it."
            );
            start_paused = true;
        }
    }

    let backend = make_backend(dry_run);

    let executor = Arc::new(Executor::spawn(backend, Os::current()));
    let cfg = ServerConfig::new(ip, computer_name(), platform());
    let (tx, mut events) = mpsc::unbounded_channel();
    let server = match Server::start(cfg, Keystore::new(store), executor, tx, start_paused).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not start the server: {e:?}");
            std::process::exit(1);
        }
    };
    println!(
        "PocketRemote listening on {} ({})",
        server.local_addr(),
        if start_paused { "PAUSED" } else { "running" }
    );
    println!(
        "{}",
        if server.paired_phone().is_some() {
            "A phone is paired."
        } else {
            "No phone paired yet: type 'pair'."
        }
    );
    println!("{HELP}");

    loop {
        tokio::select! {
            ev = events.recv() => {
                let Some(ev) = ev else { break };
                match ev {
                    ServerEvent::PhoneConnected => println!("[phone connected]"),
                    ServerEvent::PhoneDisconnected => println!("[phone disconnected]"),
                    ServerEvent::PairingAttempt => println!("[a phone is trying to pair]"),
                    ServerEvent::ApprovalRequested { code } => {
                        println!("\nPair your phone? Code {}   (type 'approve' or 'reject')", grouped(&code));
                    }
                    ServerEvent::PairingFinished { approved } => {
                        println!("[pairing {}]", if approved { "approved: any previous phone is revoked" } else { "ended without approval" });
                    }
                }
            }
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                match line.trim() {
                    "pair" => match server.open_pairing() {
                        Ok(offer) => {
                            println!("Scan with PocketRemote within {} seconds:", offer.expires_in.as_secs());
                            print_qr(&offer.uri);
                            if print_uri {
                                // Development only: the link contains the one-time pairing token.
                                println!("DEV-URI {}", offer.uri);
                            }
                        }
                        Err(e) => println!("cannot open pairing: {e:?}"),
                    },
                    "approve" => println!("{}", match server.approve() { Ok(()) => "approved".to_string(), Err(e) => format!("nothing to approve ({e:?})") }),
                    "reject" => { server.reject(); println!("rejected"); }
                    "disconnect" => server.disconnect(),
                    "revoke" => println!("{}", if server.revoke_phone().is_ok() { "phone revoked" } else { "could not revoke" }),
                    "pause" => { server.pause(); println!("paused"); }
                    "resume" => { server.resume(); println!("running"); }
                    "status" => println!(
                        "{} | paired: {} | {}",
                        server.local_addr(),
                        server.paired_phone().is_some(),
                        if server.is_paused() { "paused" } else { "running" }
                    ),
                    "quit" | "exit" => break,
                    "help" | "" => println!("{HELP}"),
                    other => println!("unknown command '{other}'. {HELP}"),
                }
            }
        }
    }
    println!("bye");
}
