//! Launch at login (macOS: a per-user LaunchAgent, no extra crates). Other systems come later.

#[cfg(target_os = "macos")]
mod imp {
    use std::path::PathBuf;

    const LABEL: &str = "com.pocketremote.host";

    fn plist_path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library/LaunchAgents")
                .join(format!("{LABEL}.plist"))
        })
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    pub fn supported() -> bool {
        true
    }

    pub fn enabled() -> bool {
        plist_path().is_some_and(|p| p.exists())
    }

    pub fn set(on: bool) -> std::io::Result<()> {
        let path = plist_path().ok_or_else(|| std::io::Error::other("no home folder"))?;
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            };
        }
        let exe = std::env::current_exe()?;
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n<key>Label</key><string>{LABEL}</string>\n<key>ProgramArguments</key><array><string>{}</string></array>\n<key>RunAtLoad</key><true/>\n</dict></plist>\n",
            escape(&exe.to_string_lossy())
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, xml)
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn supported() -> bool {
        false
    }
    pub fn enabled() -> bool {
        false
    }
    pub fn set(_on: bool) -> std::io::Result<()> {
        Ok(())
    }
}

pub use imp::{enabled, set, supported};
