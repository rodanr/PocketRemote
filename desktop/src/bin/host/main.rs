//! PocketRemote host app: menu-bar icon plus Control Panel (PLAN.md section 5). The server, executor
//! and every security rule are the existing, tested library code; this file is only the UI around them.
//! Never shows or logs keys, tokens, typed text or decrypted payloads.
#![forbid(unsafe_code)]

mod engine;
mod login;
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

// ---- Aisle Light palettes (docs/design-brief.md; reviewed with Opus). The panel follows the system
// light/dark setting. Buttons carry a 3:1 edge so they read as buttons; amber stays the lead action.
#[derive(Clone, Copy)]
struct Pal {
    ground: Color,
    card: Color,
    card_border: Color,
    text: Color,
    muted: Color,
    /// Amber used as text (code digits): darker in light mode so it stays readable.
    amber_text: Color,
    ok: Color,
    danger: Color,
    btn: Color,
    btn_hover: Color,
    btn_pressed: Color,
    btn_border: Color,
    lead_border: Color,
    track: Color,
    divider: Color,
}

const AMBER: Color = Color::from_rgb(1.0, 0.706, 0.329); // #ffb454
const ON_AMBER: Color = Color::from_rgb(0.102, 0.071, 0.031); // #1a1208

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn pal(dark: bool) -> Pal {
    if dark {
        Pal {
            ground: rgb(0x1c, 0x1b, 0x19),
            card: rgb(0x26, 0x24, 0x21),
            card_border: rgb(0x34, 0x31, 0x2c),
            text: rgb(0xf2, 0xee, 0xe8),
            muted: rgb(0xb0, 0xa0, 0x8a),
            amber_text: AMBER,
            ok: rgb(0x5f, 0xd3, 0x8d),
            danger: rgb(0xff, 0x7a, 0x6a),
            btn: rgb(0x46, 0x42, 0x3c),
            btn_hover: rgb(0x4d, 0x49, 0x43),
            btn_pressed: rgb(0x52, 0x4d, 0x46),
            btn_border: rgb(0x75, 0x6d, 0x61),
            lead_border: rgb(0xc9, 0x8a, 0x2e),
            track: rgb(0x34, 0x31, 0x2c),
            divider: rgb(0x34, 0x31, 0x2c),
        }
    } else {
        Pal {
            ground: rgb(0xf4, 0xf1, 0xec),
            card: rgb(0xff, 0xff, 0xff),
            card_border: rgb(0xdc, 0xd6, 0xcc),
            text: rgb(0x1f, 0x1c, 0x18),
            muted: rgb(0x6b, 0x61, 0x52),
            amber_text: rgb(0x9a, 0x5a, 0x00),
            ok: rgb(0x1f, 0x8a, 0x4c),
            danger: rgb(0xc2, 0x38, 0x2b),
            btn: rgb(0xff, 0xff, 0xff),
            btn_hover: rgb(0xf3, 0xef, 0xe9),
            btn_pressed: rgb(0xeb, 0xe6, 0xde),
            btn_border: rgb(0x8f, 0x85, 0x77),
            lead_border: rgb(0xb9, 0x7a, 0x1f),
            track: rgb(0xeb, 0xe6, 0xde),
            divider: rgb(0xe6, 0xe0, 0xd6),
        }
    }
}

/// Only one copy may run: two would fight over the port and the paired phone.
fn single_instance() -> Option<std::fs::File> {
    let dir = pocketremote::paths::config_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let file = std::fs::File::create(dir.join("host.lock")).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

/// Manrope (free, SIL Open Font License; see assets/fonts/OFL.txt), the same family as the phone app.
const MANROPE: iced::Font = iced::Font::with_name("Manrope");

fn main() -> iced::Result {
    let Some(_lock) = single_instance() else {
        eprintln!("PocketRemote is already running. Use its menu-bar icon.");
        return Ok(());
    };
    iced::daemon(boot, update, view)
        .title(|_: &App, _| "PocketRemote".to_string())
        .font(include_bytes!("../../../assets/fonts/Manrope_400Regular.ttf").as_slice())
        .font(include_bytes!("../../../assets/fonts/Manrope_500Medium.ttf").as_slice())
        .font(include_bytes!("../../../assets/fonts/Manrope_600SemiBold.ttf").as_slice())
        .default_font(MANROPE)
        .theme(|app: &App, _| theme(app.dark))
        .subscription(subscription)
        .run()
}

fn theme(dark: bool) -> Theme {
    let p = pal(dark);
    Theme::custom(
        "Aisle Light".to_string(),
        iced::theme::Palette {
            background: p.ground,
            text: p.text,
            primary: AMBER,
            success: p.ok,
            warning: AMBER,
            danger: p.danger,
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
    at_login: bool,
    dark: bool,
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
    ToggleLogin,
    SystemTheme(iced::theme::Mode),
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
        at_login: login::enabled(),
        dark: true,
        notice: None,
    };
    // The tray must be created on the main thread after the event loop has started.
    (app, Task::done(Msg::Init))
}

fn subscription(_: &App) -> Subscription<Msg> {
    Subscription::batch([
        Subscription::run(engine::worker).map(Msg::Engine),
        window::close_events().map(Msg::Closed),
        iced::system::theme_changes().map(Msg::SystemTheme),
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
                    size: iced::Size::new(400.0, 560.0),
                    resizable: false,
                    ..window::Settings::default()
                });
                task.map(Msg::Opened)
            }
        }
    }

    /// Re-read whether a phone is paired. This touches the Keychain, so call it only after an event
    /// that can change it: start, a finished pairing, or removing the phone.
    fn sync_paired(&mut self) {
        if let Some(server) = self.server() {
            self.paired = server.paired_phone().is_some();
        }
    }

    fn refresh(&mut self) {
        // Cheap, in-memory only: this runs 4 times a second. The paired state lives in the
        // Keychain, so it is read only when something changes (see sync_paired), never here.
        if let Some(server) = self.server() {
            self.paused = server.is_paused();
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
            // First run, or an unfinished setup: show the panel. Also ask for the system light/dark setting.
            return Task::batch([
                app.open_panel(),
                iced::system::theme().map(Msg::SystemTheme),
            ]);
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
            app.sync_paired();
            app.refresh();
        }
        Msg::Engine(Engine::NoPermission) => app.phase = Phase::NoPermission,
        Msg::Engine(Engine::Failed(m)) => app.phase = Phase::Failed(m),
        Msg::Engine(Engine::Event(ev)) => match ev {
            ServerEvent::PhoneConnected => {
                app.connected = true;
                notify("Phone connected");
            }
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
                app.sync_paired();
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
                    Err(e) => {
                        app.notice = Some(
                            match e {
                                pocketremote::server::PairError::Paused => {
                                    "Resume (or allow this network) first, then pair."
                                }
                                pocketremote::server::PairError::Keystore => {
                                    "Could not read the stored keys."
                                }
                                pocketremote::server::PairError::Internal => {
                                    "Pairing could not start. Try again."
                                }
                            }
                            .into(),
                        );
                    }
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
                app.sync_paired();
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
        Msg::ToggleLogin => match login::set(!app.at_login) {
            Ok(()) => app.at_login = login::enabled(),
            Err(_) => app.notice = Some("Could not change the launch-at-login setting.".into()),
        },
        Msg::SystemTheme(mode) => app.dark = !matches!(mode, iced::theme::Mode::Light),
        Msg::Quit => return iced::exit(),
    }
    app.refresh();
    Task::none()
}

/// A plain macOS notification. Fixed text only: nothing from the session is ever shown or logged.
fn notify(message: &'static str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("osascript")
        .args([
            "-e",
            &format!("display notification \"{message}\" with title \"PocketRemote\""),
        ])
        .spawn();
    #[cfg(not(target_os = "macos"))]
    let _ = message;
}

// ------------------------------------------------------------------ view

#[derive(Clone, Copy)]
enum Kind {
    Lead,
    Normal,
    /// Text-only, red: a rare, destructive action that should not shout.
    Quiet,
}

fn semibold() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Semibold,
        ..MANROPE
    }
}

fn medium() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Medium,
        ..MANROPE
    }
}

/// A real button: filled, with a visible edge. Hugs its label (Mac style), 30 px high.
fn btn<'a>(p: Pal, label: &'a str, kind: Kind, msg: Msg) -> Element<'a, Msg> {
    button(text(label).size(13).font(medium()))
        .padding([7, 16])
        .on_press(msg)
        .style(move |_, status| {
            let hover = matches!(status, button::Status::Hovered);
            let down = matches!(status, button::Status::Pressed);
            let (bg, fg, border) = match kind {
                Kind::Lead => (
                    if down {
                        rgb(0xe8, 0x9f, 0x3e)
                    } else if hover {
                        rgb(0xff, 0xc0, 0x70)
                    } else {
                        AMBER
                    },
                    ON_AMBER,
                    p.lead_border,
                ),
                Kind::Normal => (
                    if down {
                        p.btn_pressed
                    } else if hover {
                        p.btn_hover
                    } else {
                        p.btn
                    },
                    p.text,
                    p.btn_border,
                ),
                Kind::Quiet => (
                    Color::TRANSPARENT,
                    if hover || down {
                        Color { a: 0.8, ..p.danger }
                    } else {
                        p.danger
                    },
                    Color::TRANSPARENT,
                ),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: border,
                },
                ..button::Style::default()
            }
        })
        .into()
}

fn card<'a>(p: Pal, content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.card)),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: p.card_border,
            },
            ..container::Style::default()
        })
        .into()
}

fn heading<'a>(p: Pal, s: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(s).size(15).font(semibold()).color(p.text)
}

fn body<'a>(p: Pal, s: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(s).size(13).color(p.muted)
}

/// Small section label above a card.
fn section<'a>(p: Pal, s: &'a str) -> iced::widget::Text<'a> {
    text(s).size(12).font(semibold()).color(p.muted)
}

fn grouped(code: &str) -> String {
    let (a, b) = code.split_at(code.len().min(3));
    format!("{a} {b}")
}

/// A round status dot (shape plus words: the words are always shown beside it).
fn dot<'a>(c: Color) -> Element<'a, Msg> {
    container(text(""))
        .width(10)
        .height(10)
        .style(move |_| container::Style {
            background: Some(Background::Color(c)),
            border: Border {
                radius: 5.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// One settings row: title and a short hint on the left, a switch on the right.
fn setting<'a>(p: Pal, title: &'a str, hint: &'a str, on: bool, msg: Msg) -> Element<'a, Msg> {
    row![
        column![
            text(title).size(13).color(p.text),
            text(hint).size(12).color(p.muted)
        ]
        .spacing(2)
        .width(Length::Fill),
        iced::widget::toggler(on).on_toggle(move |_| msg.clone()),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn rule<'a>(p: Pal) -> Element<'a, Msg> {
    container(text(""))
        .width(Length::Fill)
        .height(1)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.divider)),
            ..container::Style::default()
        })
        .into()
}

/// Buttons in one row, aligned right (Mac style), the lead one last.
fn actions<'a>(items: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
    container(row(items).spacing(8))
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .into()
}

/// Title, one calm sentence, and the colour of the dot.
fn hero(app: &App, p: Pal) -> (String, String, Color) {
    match &app.phase {
        Phase::Starting => ("Starting…".into(), "One moment.".into(), p.muted),
        Phase::NoPermission => (
            "Needs permission".into(),
            "One step in System Settings.".into(),
            p.amber_text,
        ),
        Phase::Failed(_) => (
            "Can't start".into(),
            "See the message below.".into(),
            p.danger,
        ),
        Phase::Running(_) if app.untrusted.is_some() => (
            "Paused: new network".into(),
            "Allow this network to continue.".into(),
            p.danger,
        ),
        Phase::Running(_) if app.paused => (
            "Paused".into(),
            "Phones can't control this computer.".into(),
            p.danger,
        ),
        Phase::Running(_) if app.connected => (
            "Connected".into(),
            "Your phone is controlling this computer.".into(),
            AMBER,
        ),
        Phase::Running(_) if app.paired => (
            "Ready".into(),
            "Open PocketRemote on your phone.".into(),
            p.ok,
        ),
        Phase::Running(_) => (
            "No phone paired".into(),
            "Pair a phone to start.".into(),
            p.muted,
        ),
    }
}

fn view(app: &App, _: window::Id) -> Element<'_, Msg> {
    let p = pal(app.dark);
    let (title, sub, tone) = hero(app, p);
    // The dot sits on the title line only, not centred on the two-line block.
    let header = row![
        container(dot(tone)).padding(iced::Padding {
            top: 7.0,
            ..iced::Padding::ZERO
        }),
        column![
            text(title).size(20).font(semibold()).color(p.text),
            text(sub).size(13).color(p.muted)
        ]
        .spacing(4)
        .width(Length::Fill)
    ]
    .spacing(10);

    let main: Element<'_, Msg> = match &app.phase {
        Phase::Starting => column![].into(),
        Phase::NoPermission => card(
            p,
            column![
                heading(p, "Allow control of this computer"),
                column![
                    body(p, "1.  Open System Settings below."),
                    body(p, "2.  Go to Privacy & Security, then Accessibility."),
                    body(
                        p,
                        "3.  Switch on PocketRemote (or the app you started it from)."
                    ),
                ]
                .spacing(4),
                actions(vec![btn(
                    p,
                    "Open System Settings",
                    Kind::Lead,
                    Msg::OpenAccessibility
                )]),
                body(p, "This page updates once it's on."),
            ]
            .spacing(16),
        ),
        Phase::Failed(m) => card(
            p,
            column![
                heading(p, "Something stopped PocketRemote"),
                body(p, m.as_str())
            ]
            .spacing(8),
        ),
        Phase::Running(s) => running(app, p, s),
    };

    let mut page = column![header, main].spacing(24);
    if let Some(n) = &app.notice {
        page = page.push(text(n.as_str()).size(12).color(p.danger));
    }
    container(page)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.ground)),
            ..container::Style::default()
        })
        .into()
}

fn running<'a>(app: &'a App, p: Pal, s: &'a engine::Started) -> Element<'a, Msg> {
    // Highest priority first: approval, then the network question, then pairing, then the overview.
    if let Some(code) = &app.approval {
        return card(
            p,
            column![
                heading(p, "Pair this phone?"),
                body(p, "Approve only if your phone shows this code."),
                container(text(grouped(code)).size(40).color(p.amber_text))
                    .center_x(Length::Fill)
                    .padding([8, 0]),
                actions(vec![
                    btn(p, "Reject", Kind::Normal, Msg::Reject),
                    btn(p, "Approve", Kind::Lead, Msg::Approve)
                ]),
            ]
            .spacing(16),
        );
    }
    if let Some(id) = &app.untrusted {
        return card(
            p,
            column![
                heading(p, "Allow phones on this network?"),
                text(id.as_str()).size(12).color(p.muted),
                body(p, "Only on a network you trust, like home Wi-Fi."),
                actions(vec![btn(
                    p,
                    "Allow this network",
                    Kind::Lead,
                    Msg::AllowNetwork
                )]),
            ]
            .spacing(12),
        );
    }
    if let Some(pairing) = &app.pairing {
        let total = 60.0_f32;
        let left = pairing
            .until
            .saturating_duration_since(Instant::now())
            .as_secs_f32();
        return card(
            p,
            column![
                heading(p, "Scan with your phone"),
                body(p, "Open PocketRemote and scan. Same Wi-Fi."),
                // The QR plate is always white so every scanner can read it, in light or dark.
                container(qr_code(&pairing.qr).total_size(210.0))
                    .padding(12)
                    .center_x(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::WHITE)),
                        border: Border {
                            radius: 10.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
                iced::widget::progress_bar(0.0..=total, left.min(total))
                    .girth(4)
                    .style(move |_| iced::widget::progress_bar::Style {
                        background: Background::Color(p.track),
                        bar: Background::Color(AMBER),
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                    }),
                body(p, format!("Expires in {} s", left.ceil() as u32)),
            ]
            .spacing(12),
        );
    }

    // Overview: phone, then settings.
    let phone_line = if app.connected {
        "Connected right now."
    } else if app.paired {
        "Paired."
    } else {
        "None yet."
    };
    let mut buttons: Vec<Element<'a, Msg>> = Vec::new();
    if app.connected {
        buttons.push(btn(p, "Disconnect", Kind::Normal, Msg::Disconnect));
    }
    buttons.push(btn(
        p,
        if app.paired {
            "Pair new phone"
        } else {
            "Pair phone"
        },
        if app.paired { Kind::Normal } else { Kind::Lead },
        Msg::Pair,
    ));
    let mut phone_row = row![].align_y(alignment::Vertical::Center);
    if app.paired {
        phone_row = phone_row.push(btn(
            p,
            if app.confirm_revoke {
                "Click again to remove"
            } else {
                "Remove phone"
            },
            Kind::Quiet,
            Msg::Revoke,
        ));
    }
    phone_row = phone_row.push(actions(buttons));

    let mut settings = column![setting(
        p,
        "Allow phone control",
        "Off blocks your phone right away.",
        !app.paused,
        Msg::TogglePause
    )]
    .spacing(12);
    if login::supported() {
        settings = settings.push(rule(p));
        settings = settings.push(setting(
            p,
            "Open at login",
            "Start when you sign in.",
            app.at_login,
            Msg::ToggleLogin,
        ));
    }

    column![
        column![
            section(p, "Phone"),
            card(
                p,
                column![text(phone_line).size(13).color(p.text), phone_row].spacing(12)
            )
        ]
        .spacing(8),
        column![section(p, "Settings"), card(p, settings)].spacing(8),
        text(format!(
            "{} · Wi-Fi only, encrypted · {}",
            s.host_name, s.addr
        ))
        .size(11)
        .color(p.muted),
    ]
    .spacing(16)
    .into()
}
