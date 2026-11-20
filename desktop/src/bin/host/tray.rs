//! Menu-bar icon: three states and the menu from PLAN.md section 5.4.

use muda::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Look {
    /// Outline ring: running, nobody connected.
    Waiting,
    /// Ring with a pause glyph.
    Paused,
    /// Filled amber disc: a phone is controlling this computer.
    Controlled,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    OpenPanel,
    TogglePause,
    Disconnect,
    Pair,
    Quit,
}

/// The logo kit's tray icons (36 px glyph, centred on a transparent 48 px canvas so it shows at about
/// 15 pt; the library scales the whole image to 22 pt), embedded as raw RGBA.
/// Regenerate with `python3 brand/build/make_tray_rgba.py`.
const SIZE: u32 = 48;

fn icon(look: Look) -> Icon {
    let rgba: &[u8] = match look {
        Look::Waiting => include_bytes!("../../../assets/tray/tray-waiting.rgba"),
        Look::Paused => include_bytes!("../../../assets/tray/tray-paused.rgba"),
        Look::Controlled => include_bytes!("../../../assets/tray/tray-controlled.rgba"),
    };
    Icon::from_rgba(rgba.to_vec(), SIZE, SIZE).expect("valid icon")
}

pub struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    open: MenuItem,
    pause: MenuItem,
    disconnect: MenuItem,
    pair: MenuItem,
    quit: MenuItem,
    look: Look,
}

impl Tray {
    /// Must be called on the main thread after the event loop has started.
    pub fn new() -> Result<Self, String> {
        let status = MenuItem::new("Starting…", false, None);
        let open = MenuItem::new("Open Control Panel", true, None);
        let pause = MenuItem::new("Pause", true, None);
        let disconnect = MenuItem::new("Disconnect", false, None);
        let pair = MenuItem::new("Pair phone", true, None);
        let quit = MenuItem::new("Quit PocketRemote", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &open,
            &pause,
            &disconnect,
            &pair,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon_templated(icon(Look::Waiting))
            .with_tooltip("PocketRemote")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            icon,
            status,
            open,
            pause,
            disconnect,
            pair,
            quit,
            look: Look::Waiting,
        })
    }

    pub fn update(
        &mut self,
        look: Look,
        status: &str,
        paused: bool,
        connected: bool,
        can_pair: bool,
    ) {
        if look != self.look {
            // Waiting and paused are monochrome (follow light/dark); the controlled state is amber.
            let _ = if look == Look::Controlled {
                self.icon.set_icon(Some(icon(look)))
            } else {
                self.icon.set_icon_templated(Some(icon(look)))
            };
            self.look = look;
        }
        self.status.set_text(status);
        self.pause.set_text(if paused { "Resume" } else { "Pause" });
        self.disconnect.set_enabled(connected);
        self.pair.set_enabled(can_pair);
    }

    /// Drain pending menu clicks.
    pub fn poll(&self) -> Vec<Action> {
        let map: [(&MenuId, Action); 5] = [
            (self.open.id(), Action::OpenPanel),
            (self.pause.id(), Action::TogglePause),
            (self.disconnect.id(), Action::Disconnect),
            (self.pair.id(), Action::Pair),
            (self.quit.id(), Action::Quit),
        ];
        let mut out = Vec::new();
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some((_, a)) = map.iter().find(|(id, _)| **id == ev.id) {
                out.push(*a);
            }
        }
        out
    }
}
