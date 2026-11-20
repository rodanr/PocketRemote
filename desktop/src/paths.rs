//! Per-user config directory, without an extra dependency.

use std::path::PathBuf;

/// Platform config directory for PocketRemote, e.g. `~/Library/Application Support/pocketremote`.
pub fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?)
            .join("Library")
            .join("Application Support")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(p) if !p.is_empty() => PathBuf::from(p),
            _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
        }
    };
    if !base.is_absolute() {
        return None;
    }
    Some(base.join("pocketremote"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn config_dir_is_absolute_and_named() {
        if let Some(p) = super::config_dir() {
            assert!(p.is_absolute());
            assert!(p.ends_with("pocketremote"));
        }
    }
}
