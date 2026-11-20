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

const N: u32 = 44; // 22 pt at 2x

fn pixels(look: Look) -> Vec<u8> {
    let mut px = vec![0u8; (N * N * 4) as usize];
    let c = (N as f32 - 1.0) / 2.0;
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let d = (dx * dx + dy * dy).sqrt();
            // 1 inside the shape, with a one-pixel soft edge.
            let edge = |inner: f32, outer: f32| {
                ((d - inner + 0.5).clamp(0.0, 1.0) * (outer - d + 0.5).clamp(0.0, 1.0))
                    .clamp(0.0, 1.0)
            };
            let (a, rgb) = match look {
                Look::Controlled => (edge(-1.0, 13.0), [255u8, 180, 84]),
                Look::Waiting => (edge(9.0, 13.0), [0, 0, 0]),
                Look::Paused => {
                    let ring = edge(9.0, 13.0);
                    let bar =
                        if (dy.abs() < 5.5) && ((dx - 3.0).abs() < 1.6 || (dx + 3.0).abs() < 1.6) {
                            1.0
                        } else {
                            0.0
                        };
                    (ring.max(bar), [0, 0, 0])
                }
            };
            let i = ((y * N + x) * 4) as usize;
            px[i..i + 3].copy_from_slice(&rgb);
            px[i + 3] = (a * 255.0) as u8;
        }
    }
    px
}

fn icon(look: Look) -> Icon {
    Icon::from_rgba(pixels(look), N, N).expect("valid icon")
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
