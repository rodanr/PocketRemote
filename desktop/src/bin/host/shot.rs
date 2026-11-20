//! Development helper: the app can take pictures of its own window (no screen-recording permission
//! needed) so the Control Panel can be reviewed, and README screenshots regenerated, without a person.
//! `POCKETREMOTE_SHOTS=<dir>` runs a short scripted tour and exits; `POCKETREMOTE_THEME=light|dark`
//! forces the theme. Pair it with `POCKETREMOTE_DRY_RUN=1 POCKETREMOTE_FILE_KEYS=1` and a scratch HOME.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Plan {
    pub dir: PathBuf,
    pub step: u8,
    pub due: Instant,
    pub theme: &'static str,
}

impl Plan {
    pub fn from_env() -> Option<Self> {
        let dir = PathBuf::from(std::env::var_os("POCKETREMOTE_SHOTS")?);
        let theme = if std::env::var("POCKETREMOTE_THEME").is_ok_and(|t| t == "light") {
            "light"
        } else {
            "dark"
        };
        Some(Self {
            dir,
            step: 0,
            due: Instant::now() + Duration::from_secs(3),
            theme,
        })
    }

    pub fn wait(&mut self, secs: f32) {
        self.due = Instant::now() + Duration::from_secs_f32(secs);
    }
}

/// Saves RGBA pixels as a PNG (through a temporary BMP converted by macOS `sips`; this is a dev tool).
pub fn save(dir: &Path, name: &str, size: iced::Size<u32>, rgba: &[u8]) {
    let (w, h) = (size.width as usize, size.height as usize);
    let row = (w * 3).div_ceil(4) * 4;
    let mut bmp = Vec::with_capacity(54 + row * h);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((54 + row * h) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(w as i32).to_le_bytes());
    bmp.extend_from_slice(&(h as i32).to_le_bytes()); // positive height: rows are stored bottom-up
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        for x in 0..w {
            let i = (y * w + x) * 4;
            bmp.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i]]);
        }
        bmp.extend(std::iter::repeat_n(0u8, row - w * 3));
    }
    let _ = std::fs::create_dir_all(dir);
    let bmp_path = dir.join(format!("{name}.bmp"));
    let png_path = dir.join(format!("{name}.png"));
    if std::fs::write(&bmp_path, bmp).is_ok() {
        let _ = std::process::Command::new("sips")
            .args(["-s", "format", "png"])
            .arg(&bmp_path)
            .arg("--out")
            .arg(&png_path)
            .output();
        let _ = std::fs::remove_file(&bmp_path);
    }
}
