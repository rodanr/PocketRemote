//! Key storage (PLAN.md §3.3). Host static keypair and the single paired phone's public key.
//!
//! Primary store is the OS keychain; fallback is a `0600` file in the config directory
//! (Unix only: Windows has no user-only-ACL fallback yet, so there the keychain is required).
//! Never log anything from this module.

use crate::noise::{PublicKey, StaticKeypair, generate_static_keypair};
use std::fmt;
use std::path::PathBuf;
use zeroize::Zeroizing;

const HOST_PRIVATE: &str = "host_private";
const HOST_PUBLIC: &str = "host_public";
const PHONE_PUBLIC: &str = "phone_public";
const SERVICE: &str = "pocketremote";

/// Opaque failure; carries no key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeystoreError {
    Unavailable,
    Failed,
}

impl fmt::Display for KeystoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => f.write_str("key store unavailable"),
            Self::Failed => f.write_str("key store operation failed"),
        }
    }
}

impl std::error::Error for KeystoreError {}

type Result<T> = std::result::Result<T, KeystoreError>;

pub trait SecretStore: Send {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn set(&self, name: &str, value: &[u8]) -> Result<()>;
    /// Deleting something that does not exist is not an error.
    fn delete(&self, name: &str) -> Result<()>;
}

/// In-memory store, for tests and for running without persistence.
#[derive(Default)]
pub struct MemoryStore(std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>);

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let map = self.0.lock().map_err(|_| KeystoreError::Failed)?;
        Ok(map.get(name).map(|v| Zeroizing::new(v.clone())))
    }
    fn set(&self, name: &str, value: &[u8]) -> Result<()> {
        let mut map = self.0.lock().map_err(|_| KeystoreError::Failed)?;
        map.insert(name.to_owned(), value.to_vec());
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<()> {
        let mut map = self.0.lock().map_err(|_| KeystoreError::Failed)?;
        map.remove(name);
        Ok(())
    }
}

/// OS keychain via `keyring-core` (macOS Keychain, Windows Credential Manager).
pub struct OsKeyring;

impl OsKeyring {
    /// Register the platform store. Fails on platforms without a supported OS store.
    pub fn init() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let store = apple_native_keyring_store::keychain::Store::new()
                .map_err(|_| KeystoreError::Unavailable)?;
            keyring_core::set_default_store(store);
            Ok(Self)
        }
        #[cfg(target_os = "windows")]
        {
            let store = windows_native_keyring_store::Store::new()
                .map_err(|_| KeystoreError::Unavailable)?;
            keyring_core::set_default_store(store);
            Ok(Self)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(KeystoreError::Unavailable)
        }
    }

    fn entry(name: &str) -> Result<keyring_core::Entry> {
        keyring_core::Entry::new(SERVICE, name).map_err(|_| KeystoreError::Failed)
    }
}

impl SecretStore for OsKeyring {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match Self::entry(name)?.get_secret() {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(_) => Err(KeystoreError::Failed),
        }
    }
    fn set(&self, name: &str, value: &[u8]) -> Result<()> {
        Self::entry(name)?
            .set_secret(value)
            .map_err(|_| KeystoreError::Failed)
    }
    fn delete(&self, name: &str) -> Result<()> {
        match Self::entry(name)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => Err(KeystoreError::Failed),
        }
    }
}

/// Fallback file store. Files are created `0600` inside a `0700` directory (Unix only).
pub struct FileStore {
    dir: PathBuf,
}

impl FileStore {
    pub fn new(dir: PathBuf) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&dir)
                .map_err(|_| KeystoreError::Failed)?;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| KeystoreError::Failed)?;
            Ok(Self { dir })
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
            Err(KeystoreError::Unavailable)
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl SecretStore for FileStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match std::fs::read(self.path(name)) {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(KeystoreError::Failed),
        }
    }

    #[cfg(unix)]
    fn set(&self, name: &str, value: &[u8]) -> Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let tmp = self.path(&format!("{name}.tmp"));
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|_| KeystoreError::Failed)?;
        f.write_all(value).map_err(|_| KeystoreError::Failed)?;
        f.sync_all().map_err(|_| KeystoreError::Failed)?;
        std::fs::rename(&tmp, self.path(name)).map_err(|_| KeystoreError::Failed)
    }

    #[cfg(not(unix))]
    fn set(&self, _name: &str, _value: &[u8]) -> Result<()> {
        Err(KeystoreError::Unavailable)
    }

    fn delete(&self, name: &str) -> Result<()> {
        match std::fs::remove_file(self.path(name)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(KeystoreError::Failed),
        }
    }
}

/// OS keychain if available, otherwise the `0600` file fallback in the config directory.
pub fn open_default_store() -> Result<Box<dyn SecretStore>> {
    if let Ok(os) = OsKeyring::init() {
        return Ok(Box::new(os));
    }
    let dir = crate::paths::config_dir()
        .ok_or(KeystoreError::Unavailable)?
        .join("keys");
    Ok(Box::new(FileStore::new(dir)?))
}

/// Host keypair and the single paired phone.
pub struct Keystore {
    store: Box<dyn SecretStore>,
}

impl Keystore {
    pub fn new(store: Box<dyn SecretStore>) -> Self {
        Self { store }
    }

    /// Load the host keypair, generating one on first use. If the stored pair is damaged,
    /// a fresh pair is generated and the paired phone is revoked, because the phone pins the
    /// old host key and would reject the new one anyway.
    pub fn host_keypair(&self) -> Result<StaticKeypair> {
        let private = self.store.get(HOST_PRIVATE)?;
        let public = self.store.get(HOST_PUBLIC)?;
        if let (Some(private), Some(public)) = (private, public)
            && let (Ok(private), Ok(public)) = (
                <[u8; 32]>::try_from(private.as_slice()),
                <[u8; 32]>::try_from(public.as_slice()),
            )
        {
            return Ok(StaticKeypair {
                private: Zeroizing::new(private),
                public,
            });
        }
        self.reset_pairing()?;
        let kp = generate_static_keypair().map_err(|_| KeystoreError::Failed)?;
        self.store.set(HOST_PRIVATE, kp.private.as_slice())?;
        self.store.set(HOST_PUBLIC, &kp.public)?;
        Ok(kp)
    }

    pub fn paired_phone(&self) -> Result<Option<PublicKey>> {
        match self.store.get(PHONE_PUBLIC)? {
            None => Ok(None),
            Some(v) => match <[u8; 32]>::try_from(v.as_slice()) {
                Ok(pk) => Ok(Some(pk)),
                // A damaged entry means no usable pairing.
                Err(_) => {
                    self.store.delete(PHONE_PUBLIC)?;
                    Ok(None)
                }
            },
        }
    }

    /// Exactly one phone: this replaces (and so revokes) any previous one.
    pub fn set_paired_phone(&self, phone_public: &PublicKey) -> Result<()> {
        self.store.set(PHONE_PUBLIC, phone_public)
    }

    pub fn revoke_phone(&self) -> Result<()> {
        self.store.delete(PHONE_PUBLIC)
    }

    /// "Reset pairing": deletes the host key and the paired phone.
    pub fn reset_pairing(&self) -> Result<()> {
        self.store.delete(PHONE_PUBLIC)?;
        self.store.delete(HOST_PRIVATE)?;
        self.store.delete(HOST_PUBLIC)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Keystore {
        Keystore::new(Box::new(MemoryStore::default()))
    }

    #[test]
    fn host_keypair_is_created_once_and_stable() {
        let ks = mem();
        let a = ks.host_keypair().unwrap();
        let b = ks.host_keypair().unwrap();
        assert_eq!(a.public, b.public);
        assert_eq!(*a.private, *b.private);
    }

    #[test]
    fn exactly_one_phone_and_new_one_replaces_old() {
        let ks = mem();
        assert_eq!(ks.paired_phone().unwrap(), None);
        ks.set_paired_phone(&[1; 32]).unwrap();
        ks.set_paired_phone(&[2; 32]).unwrap();
        assert_eq!(ks.paired_phone().unwrap(), Some([2; 32]));
        ks.revoke_phone().unwrap();
        assert_eq!(ks.paired_phone().unwrap(), None);
    }

    #[test]
    fn reset_pairing_deletes_host_key_and_phone() {
        let ks = mem();
        let before = ks.host_keypair().unwrap();
        ks.set_paired_phone(&[9; 32]).unwrap();
        ks.reset_pairing().unwrap();
        assert_eq!(ks.paired_phone().unwrap(), None);
        let after = ks.host_keypair().unwrap();
        assert_ne!(before.public, after.public);
    }

    #[test]
    fn damaged_host_key_regenerates_and_revokes_phone() {
        let store = MemoryStore::default();
        store.set(HOST_PRIVATE, &[1, 2, 3]).unwrap();
        store.set(HOST_PUBLIC, &[4; 32]).unwrap();
        store.set(PHONE_PUBLIC, &[5; 32]).unwrap();
        let ks = Keystore::new(Box::new(store));
        let kp = ks.host_keypair().unwrap();
        assert_ne!(kp.public, [4; 32]);
        assert_eq!(ks.paired_phone().unwrap(), None);
    }

    #[test]
    fn damaged_phone_entry_counts_as_unpaired() {
        let store = MemoryStore::default();
        store.set(PHONE_PUBLIC, &[5; 7]).unwrap();
        let ks = Keystore::new(Box::new(store));
        assert_eq!(ks.paired_phone().unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn file_store_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "pocketremote-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let fs = FileStore::new(dir.clone()).unwrap();
        fs.set("k", b"secret").unwrap();
        assert_eq!(fs.get("k").unwrap().unwrap().as_slice(), b"secret");
        let file_mode = std::fs::metadata(dir.join("k"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600);
        assert_eq!(dir_mode, 0o700);
        fs.delete("k").unwrap();
        assert!(fs.get("k").unwrap().is_none());
        fs.delete("k").unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
