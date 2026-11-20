//! PocketRemote host app: menu-bar icon plus Control Panel (PLAN.md section 5). The server, executor
//! and every security rule are the existing, tested library code; this file is only the UI around them.
//! Never shows or logs keys, tokens, typed text or decrypted payloads.
#![forbid(unsafe_code)]

mod engine;
mod tray;

use engine::Engine;
use iced::widget::{button, column, container, qr_code, row, text};
use iced::{
    Background, Border, Color, Element, Length, Subscription, Task, Theme, alignment, time, window,
};
use pocketremote::net::NetworkId;
use pocketremote::server::ServerEvent;
use std::time::{Duration, Instant};
use tray::{Action, Look, Tray};

// ---- Aisle Light colours (docs/design-brief.md) ----
const GROUND: Color = Color::from_rgb(0.027, 0.027, 0.039); // #07070a
const SURFACE: Color = Color::from_rgb(0.110, 0.098, 0.078); // #1c1914
const PRESSED: Color = Color::from_rgb(0.165, 0.145, 0.110); // #2a251c
const RIM: Color = Color::from_rgb(0.149, 0.133, 0.098); // #262219
const TEXT: Color = Color::from_rgb(0.910, 0.886, 0.839); // #e8e2d6
const MUTED: Color = Color::from_rgb(0.627, 0.541, 0.416); // #a08a6a
const AMBER: Color = Color::from_rgb(1.0, 0.706, 0.329); // #ffb454
const OK: Color = Color::from_rgb(0.373, 0.827, 0.553); // #5fd38d
const DANGER: Color = Color::from_rgb(1.0, 0.420, 0.353); // #ff6b5a
const ON_AMBER: Color = Color::from_rgb(0.067, 0.047, 0.020);

fn main() -> iced::Result {
    iced::daemon(boot, update, view)
        .title(|_: &App, _| "PocketRemote".to_string())
        .theme(|_: &App, _| theme())
        .subscription(subscription)
        .run()
}

fn theme() -> Theme {
    Theme::custom(
        "Aisle Light".to_string(),
        iced::theme::Palette {
            background: GROUND,
            text: TEXT,
            primary: AMBER,
            success: OK,
            warning: AMBER,
            danger: DANGER,
        },
    )
}

enum Phase {
    Starting,
    NoPermission,
    Failed(String),
    Running(engine::Started),
}

struct Pairing {
    qr: qr_code::Data,
    until: Instant,
}

struct App {
    phase: Phase,
    tray: Option<Tray>,
    win: Option<window::Id>,
    connected: bool,
    paused: bool,
    paired: bool,
    untrusted: Option<NetworkId>,
    pairing: Option<Pairing>,
    approval: Option<String>,
    confirm_revoke: bool,
    notice: Option<String>,
}

#[derive(Debug, Clone)]
enum Msg {
    Init,
    Tick,
    Engine(Engine),
    Opened(window::Id),
    Closed(window::Id),
    OpenPanel,
    Pair,
    Approve,
    Reject,
    Disconnect,
    Revoke,
    TogglePause,
    AllowNetwork,
    OpenAccessibility,
    Quit,
}

fn boot() -> (App, Task<Msg>) {
    let app = App {
        phase: Phase::Starting,
        tray: None,
        win: None,
        connected: false,
        paused: false,
        paired: false,
        untrusted: None,
        pairing: None,
        approval: None,
        confirm_revoke: false,
        notice: None,
    };
    // The tray must be created on the main thread after the event loop has started.
    (app, Task::done(Msg::Init))
}

fn subscription(_: &App) -> Subscription<Msg> {
    Subscription::batch([
        Subscription::run(engine::worker).map(Msg::Engine),
        window::close_events().map(Msg::Closed),
        time::every(Duration::from_millis(250)).map(|_| Msg::Tick),
    ])
}

impl App {
    fn server(&self) -> Option<&pocketremote::server::Server> {
        match &self.phase {
            Phase::Running(s) => Some(&s.server),
            _ => None,
        }
    }

    fn look(&self) -> Look {
        if self.paused || self.untrusted.is_some() {
            Look::Paused
        } else if self.connected {
            Look::Controlled
        } else {
            Look::Waiting
        }
    }

    fn status(&self) -> String {
        match &self.phase {
            Phase::Starting => "Starting…".into(),
            Phase::NoPermission => "Needs permission".into(),
            Phase::Failed(_) => "Can't start".into(),
            Phase::Running(_) if self.untrusted.is_some() => "Paused: new network".into(),
            Phase::Running(_) if self.paused => "Paused".into(),
            Phase::Running(_) if self.connected => "Phone connected".into(),
            Phase::Running(_) if self.paired => "Waiting for your phone".into(),
            Phase::Running(_) => "No phone paired yet".into(),
        }
    }

    fn open_panel(&self) -> Task<Msg> {
        match self.win {
            Some(id) => window::gain_focus(id),
            None => {
                let (_, task) = window::open(window::Settings {
                    size: iced::Size::new(440.0, 640.0),
                    resizable: false,
                    ..window::Settings::default()
                });
                task.map(Msg::Opened)
            }
        }
    }

    fn refresh(&mut self) {
        if let Some(server) = self.server() {
            let (paused, paired) = (server.is_paused(), server.paired_phone().is_some());
            self.paused = paused;
            self.paired = paired;
        }
        if self
            .pairing
            .as_ref()
            .is_some_and(|p| Instant::now() >= p.until)
        {
            self.pairing = None;
        }
        let status = self.status();
        let look = self.look();
        let (paused, connected) = (self.paused || self.untrusted.is_some(), self.connected);
        let can_pair = matches!(self.phase, Phase::Running(_)) && self.untrusted.is_none();
        if let Some(tray) = &mut self.tray {
            tray.update(look, &status, paused, connected, can_pair);
        }
    }
}

fn update(app: &mut App, msg: Msg) -> Task<Msg> {
    match msg {
        Msg::Init => {
            match Tray::new() {
                Ok(t) => app.tray = Some(t),
                Err(e) => app.notice = Some(format!("Menu-bar icon unavailable: {e}")),
            }
            // First run, or an unfinished setup: show the panel.
            return app.open_panel();
        }
        Msg::Tick => {
            app.refresh();
            let actions = app.tray.as_ref().map(Tray::poll).unwrap_or_default();
            let tasks: Vec<_> = actions
                .into_iter()
                .map(|a| {
                    update(
                        app,
                        match a {
                            Action::OpenPanel => Msg::OpenPanel,
                            Action::TogglePause => Msg::TogglePause,
                            Action::Disconnect => Msg::Disconnect,
                            Action::Pair => Msg::Pair,
                            Action::Quit => Msg::Quit,
                        },
                    )
                })
                .collect();
            return Task::batch(tasks);
        }
        Msg::Engine(Engine::Started(s)) => {
            app.untrusted = s.untrusted.clone();
            app.phase = Phase::Running(s);
            app.refresh();
        }
        Msg::Engine(Engine::NoPermission) => app.phase = Phase::NoPermission,
        Msg::Engine(Engine::Failed(m)) => app.phase = Phase::Failed(m),
        Msg::Engine(Engine::Event(ev)) => match ev {
            ServerEvent::PhoneConnected => app.connected = true,
            ServerEvent::PhoneDisconnected => app.connected = false,
            ServerEvent::PairingAttempt => {}
            ServerEvent::ApprovalRequested { code } => {
                app.approval = Some(code);
                // Only the Approve step may take focus.
                return app.open_panel();
            }
            ServerEvent::PairingFinished { .. } => {
                app.approval = None;
                app.pairing = None;
                app.refresh();
            }
        },
        Msg::Opened(id) => app.win = Some(id),
        Msg::Closed(id) => {
            if app.win == Some(id) {
                app.win = None;
            }
        }
        Msg::OpenPanel => return app.open_panel(),
        Msg::Pair => {
            if let Some(server) = app.server() {
                match server.open_pairing() {
                    Ok(offer) => match qr_code::Data::new(&offer.uri) {
                        Ok(qr) => {
                            app.pairing = Some(Pairing {
                                qr,
                                until: Instant::now() + offer.expires_in,
                            });
                            app.notice = None;
                        }
                        Err(_) => app.notice = Some("Could not draw the pairing code.".into()),
                    },
                    Err(_) => app.notice = Some("Allow this network first, then pair.".into()),
                }
            }
            return app.open_panel();
        }
        Msg::Approve => {
            if let Some(server) = app.server() {
                let _ = server.approve();
            }
            app.approval = None;
        }
        Msg::Reject => {
            if let Some(server) = app.server() {
                server.reject();
            }
            app.approval = None;
        }
        Msg::Disconnect => {
            if let Some(server) = app.server() {
                server.disconnect();
            }
        }
        Msg::Revoke => {
            if app.confirm_revoke {
                if let Some(server) = app.server() {
                    let _ = server.revoke_phone();
                }
                app.confirm_revoke = false;
            } else {
                app.confirm_revoke = true;
            }
        }
        Msg::TogglePause => {
            if let Some(server) = app.server() {
                if server.is_paused() {
                    server.resume();
                } else {
                    server.pause();
                }
            }
        }
        Msg::AllowNetwork => {
            if let (Some(id), Some(server)) = (app.untrusted.take(), app.server()) {
                engine::trust_network(id);
                server.resume();
            }
        }
        Msg::OpenAccessibility => {
            // Opens the Privacy > Accessibility pane. The engine re-checks every 2 seconds.
            let _ = std::process::Command::new("open")
                .arg(
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
                )
                .spawn();
        }
        Msg::Quit => return iced::exit(),
    }
    app.refresh();
    Task::none()
}

// ------------------------------------------------------------------ view

#[derive(Clone, Copy)]
enum Kind {
    Lead,
    Normal,
    Danger,
}

fn btn(label: &str, kind: Kind, msg: Msg) -> Element<'_, Msg> {
    button(
        text(label)
            .size(15)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center),
    )
    .padding([13, 18])
    .width(Length::Fill)
    .on_press(msg)
    .style(move |_, status| {
        let pressed = matches!(status, button::Status::Pressed | button::Status::Hovered);
        let (bg, fg, border) = match kind {
            Kind::Lead => (
                if pressed {
                    Color::from_rgb(0.91, 0.624, 0.243)
                } else {
                    AMBER
                },
                ON_AMBER,
                Color::from_rgba(1.0, 1.0, 1.0, 0.18),
            ),
            Kind::Normal => (if pressed { PRESSED } else { SURFACE }, TEXT, RIM),
            Kind::Danger => (
                if pressed {
                    Color::from_rgba(1.0, 0.42, 0.353, 0.14)
                } else {
                    SURFACE
                },
                DANGER,
                Color::from_rgba(1.0, 0.42, 0.353, 0.45),
            ),
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: fg,
            border: Border {
                radius: 16.0.into(),
                width: 1.0,
                color: border,
            },
            ..button::Style::default()
        }
    })
    .into()
}

fn card<'a>(content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    container(content)
        .padding(20)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(SURFACE)),
            border: Border {
                radius: 20.0.into(),
                width: 1.0,
                color: RIM,
            },
            ..container::Style::default()
        })
        .into()
}

fn heading<'a>(s: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(s).size(20).color(TEXT)
}

fn body<'a>(s: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(s).size(14).color(MUTED)
}

fn grouped(code: &str) -> String {
    let (a, b) = code.split_at(code.len().min(3));
    format!("{a} {b}")
}

fn view(app: &App, _: window::Id) -> Element<'_, Msg> {
    let dot = match app.look() {
        Look::Controlled => AMBER,
        Look::Paused => DANGER,
        Look::Waiting if matches!(app.phase, Phase::Running(_)) => OK,
        Look::Waiting => MUTED,
    };
    let name = match &app.phase {
        Phase::Running(s) => s.host_name.clone(),
        _ => "PocketRemote".to_string(),
    };
    let header = row![
        text("●").size(16).color(dot),
        column![
            text(name).size(22).color(TEXT),
            text(app.status()).size(14).color(MUTED)
        ]
        .spacing(2)
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let main: Element<'_, Msg> = match &app.phase {
        Phase::Starting => card(body("Starting…")),
        Phase::NoPermission => card(
            column![
                heading("Allow PocketRemote to control this computer"),
                body("macOS needs your OK before an app can move the pointer or press keys. Open System Settings, turn on PocketRemote under Privacy & Security, then Accessibility. This page notices on its own."),
                btn("Open System Settings", Kind::Lead, Msg::OpenAccessibility),
            ]
            .spacing(14),
        ),
        Phase::Failed(m) => card(column![heading("PocketRemote can't start"), body(m)].spacing(10)),
        Phase::Running(s) => running(app, s),
    };

    let mut page = column![header, main].spacing(20);
    if let Some(n) = &app.notice {
        page = page.push(text(n).size(13).color(DANGER));
    }
    container(page)
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(GROUND)),
            ..container::Style::default()
        })
        .into()
}

fn running<'a>(app: &'a App, s: &'a engine::Started) -> Element<'a, Msg> {
    // Highest priority first: approval, then the network question, then pairing, then the overview.
    if let Some(code) = &app.approval {
        return card(
            column![
                heading("Pair your phone?"),
                body("A phone is asking to control this computer. Check that this code matches the one on the phone."),
                text(grouped(code)).size(40).color(AMBER),
                row![btn("Reject", Kind::Normal, Msg::Reject), btn("Approve", Kind::Lead, Msg::Approve)].spacing(10),
            ]
            .spacing(14),
        );
    }
    if let Some(id) = &app.untrusted {
        return card(
            column![
                heading("Allow phones on this network?"),
                body(id.as_str()),
                body("PocketRemote stays paused until you say yes. Only do this on a network you trust, such as your home Wi-Fi."),
                btn("Allow", Kind::Lead, Msg::AllowNetwork),
            ]
            .spacing(14),
        );
    }
    if let Some(p) = &app.pairing {
        let left = p.until.saturating_duration_since(Instant::now()).as_secs();
        return card(
            column![
                heading("Scan with PocketRemote"),
                body(
                    "Open the app on your phone and point it at this code. Both on the same Wi-Fi."
                ),
                container(qr_code(&p.qr).total_size(260.0))
                    .padding(14)
                    .center_x(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.96, 0.95, 0.92))),
                        border: Border {
                            radius: 16.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
                body(format!("Expires in {left} s")),
            ]
            .spacing(14),
        );
    }

    let phone = if app.connected {
        "A phone is connected."
    } else if app.paired {
        "A phone is paired. It connects when you open the app."
    } else {
        "No phone paired yet."
    };
    let mut col = column![
        heading("Your phone"),
        body(phone),
        body(format!("Listening on {}", s.addr))
    ]
    .spacing(10);
    col = col.push(btn(
        if app.paired {
            "Pair a different phone"
        } else {
            "Pair phone"
        },
        if app.paired { Kind::Normal } else { Kind::Lead },
        Msg::Pair,
    ));
    if app.connected {
        col = col.push(btn("Disconnect", Kind::Normal, Msg::Disconnect));
    }
    if app.paired {
        col = col.push(btn(
            if app.confirm_revoke {
                "Tap again to remove this phone"
            } else {
                "Remove phone"
            },
            Kind::Danger,
            Msg::Revoke,
        ));
    }
    col = col.push(btn(
        if app.paused { "Resume" } else { "Pause" },
        Kind::Normal,
        Msg::TogglePause,
    ));
    card(col.spacing(12))
}
