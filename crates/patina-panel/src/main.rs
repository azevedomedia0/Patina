//! Patina top panel: Milestone 0 toolkit spike.
//!
//! A 36 px layer-shell panel drawn with iced, styled only from `patina-tokens`.

mod motion;

use std::{
    path::Path,
    time::{Duration, Instant},
};

use iced::font::{Family, Weight};
use iced::futures::{SinkExt, Stream};
use iced::widget::{button, column, container, row, stack, svg, text, Space};
use iced::{
    alignment, Alignment, Background, Border, Color as IcedColor, Element, Font, Length, Radians,
    Rotation, Subscription, Task,
};
use iced_layershell::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use patina_sysinfo::{battery, clock, Battery};
use patina_tokens as tok;

use motion::CubicBezier;

const SANS: &[u8] =
    include_bytes!("../../../assets/fonts/plus-jakarta-sans/PlusJakartaSans-Variable.ttf");
const MONO: &[u8] =
    include_bytes!("../../../assets/fonts/jetbrains-mono/JetBrainsMono-Variable.ttf");

/// Lucide icons (ISC licence, `assets/icons/lucide/LICENSE`), stroke 2, 24 px grid.
mod icons {
    pub const LAYOUT_GRID: &[u8] = include_bytes!("../../../assets/icons/lucide/layout-grid.svg");
    pub const WIFI: &[u8] = include_bytes!("../../../assets/icons/lucide/wifi.svg");
    pub const BLUETOOTH: &[u8] = include_bytes!("../../../assets/icons/lucide/bluetooth.svg");
    pub const SUN: &[u8] = include_bytes!("../../../assets/icons/lucide/sun.svg");
    pub const VOLUME_2: &[u8] = include_bytes!("../../../assets/icons/lucide/volume-2.svg");
    pub const BATTERY_FULL: &[u8] = include_bytes!("../../../assets/icons/lucide/battery-full.svg");
    pub const BELL: &[u8] = include_bytes!("../../../assets/icons/lucide/bell.svg");
}

/// A 14 px tinted icon, as used throughout the top panel.
fn icon(bytes: &'static [u8], color: IcedColor) -> svg::Svg<'static> {
    svg(svg::Handle::from_memory(bytes))
        .width(14)
        .height(14)
        .style(move |_, _| svg::Style { color: Some(color) })
}

fn sans(weight: Weight) -> Font {
    Font {
        family: Family::Name("Plus Jakarta Sans"),
        weight,
        ..Font::DEFAULT
    }
}

fn mono(weight: Weight) -> Font {
    Font {
        family: Family::Name("JetBrains Mono"),
        weight,
        ..Font::DEFAULT
    }
}

fn c(t: tok::Color) -> IcedColor {
    IcedColor::from_rgba8(t.r, t.g, t.b, t.a)
}

fn lerp(a: IcedColor, b: IcedColor, t: f32) -> IcedColor {
    IcedColor {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

struct Panel {
    /// Local time as the panel shows it, refreshed on the minute.
    clock: String,
    /// `None` on machines without a battery: the battery is then not shown at all.
    battery: Option<Battery>,
    /// Unread notifications. Nothing delivers notifications yet, so this stays zero.
    unread: u32,
    /// Title of the focused window, when there is one.
    active_title: Option<String>,
    workspace: u8,
    previous: u8,
    started: Option<Instant>,
    now: Instant,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Workspace(u8),
    Frame(Instant),
    Launcher,
    /// The displayed minute changed.
    Clock,
    /// Time to look at the battery again.
    Battery,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            clock: clock::now(),
            battery: battery::read(Path::new(battery::SYSFS)),
            unread: 0,
            active_title: None,
            workspace: 1,
            previous: 1,
            started: None,
            now: Instant::now(),
        }
    }
}

impl Panel {
    /// Eased 0..=1 progress of the workspace highlight, or 1 when idle.
    fn progress(&self) -> f32 {
        match self.started {
            Some(s) => {
                let t =
                    self.now.saturating_duration_since(s).as_secs_f32() / tok::DUR.as_secs_f32();
                CubicBezier::PATINA.ease(t)
            }
            None => 1.0,
        }
    }

    fn animating(&self) -> bool {
        self.started
            .is_some_and(|s| self.now.saturating_duration_since(s) < tok::DUR)
    }
}

fn namespace() -> String {
    // The compositor keys glass/blur and stacking behaviour on this namespace.
    String::from("patina-panel")
}

fn update(panel: &mut Panel, message: Message) -> Task<Message> {
    match message {
        Message::Workspace(n) if n != panel.workspace => {
            panel.previous = panel.workspace;
            panel.workspace = n;
            panel.now = Instant::now();
            panel.started = Some(panel.now);
        }
        Message::Frame(now) => {
            panel.now = now;
            if !panel.animating() {
                panel.started = None;
            }
        }
        Message::Clock => panel.clock = clock::now(),
        Message::Battery => panel.battery = battery::read(Path::new(battery::SYSFS)),
        Message::Workspace(_) | Message::Launcher => {}
        _ => unreachable!(),
    }
    Task::none()
}

/// One message at the start of every minute, so the clock redraws on the minute and the panel
/// sleeps in between.
fn minute_ticks() -> impl Stream<Item = Message> {
    iced::stream::channel(1, async |mut output| loop {
        tokio::time::sleep(clock::until_next_minute(&chrono::Local::now())).await;
        let _ = output.send(Message::Clock).await;
    })
}

fn subscription(panel: &Panel) -> Subscription<Message> {
    let animation = if panel.animating() {
        iced::time::every(Duration::from_millis(8)).map(Message::Frame)
    } else {
        Subscription::none()
    };
    Subscription::batch([
        animation,
        Subscription::run(minute_ticks),
        // Charge changes slowly; looking twice a minute is plenty and costs nothing when there
        // is no battery.
        iced::time::every(Duration::from_secs(30)).map(|_| Message::Battery),
    ])
}

fn workspace_button(panel: &Panel, n: u8) -> Element<'_, Message> {
    let p = panel.progress();
    let weight = if n == panel.workspace {
        p
    } else if n == panel.previous && panel.started.is_some() {
        1.0 - p
    } else {
        0.0
    };
    let bg = IcedColor {
        a: c(tok::SURFACE_ACTIVE).a * weight,
        ..c(tok::SURFACE_ACTIVE)
    };
    let fg = lerp(c(tok::TEXT_MUTED), c(tok::ACCENT_TEXT), weight);
    button(
        container(
            text(n.to_string())
                .size(11)
                .font(mono(Weight::Semibold))
                .color(fg),
        )
        .center(Length::Fill),
    )
    .width(22)
    .height(22)
    .padding(0)
    .on_press(Message::Workspace(n))
    .style(move |_, _| button::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 11.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    })
    .into()
}

fn view(panel: &Panel) -> Element<'_, Message> {
    let launcher =
        button(container(icon(icons::LAYOUT_GRID, c(tok::TEXT_ON_ACCENT))).center(Length::Fill))
            .height(24)
            .width(32)
            .padding(0)
            .on_press(Message::Launcher)
            .style(|_, _| button::Style {
                background: Some(Background::Color(c(tok::ACCENT))),
                text_color: c(tok::TEXT_ON_ACCENT),
                border: Border {
                    radius: tok::RADIUS_PILL.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            });

    let workspaces = row((1..=4).map(|n| workspace_button(panel, n))).spacing(2);
    let mut left = row![launcher, workspaces]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    if let Some(title) = &panel.active_title {
        left = left.push(
            text(title.as_str())
                .size(tok::FS_SMALL)
                .font(sans(Weight::Semibold))
                .color(c(tok::TEXT_PRIMARY)),
        );
    }
    let clock = text(panel.clock.as_str())
        .size(tok::FS_SMALL)
        .font(mono(Weight::Medium))
        .color(c(tok::TEXT_PRIMARY));
    let dim = c(tok::TEXT_SECONDARY);
    let mut status = row![
        icon(icons::WIFI, dim),
        icon(icons::BLUETOOTH, dim),
        icon(icons::SUN, dim),
        icon(icons::VOLUME_2, dim),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if let Some(battery) = panel.battery {
        status = status.push(
            row![
                icon(icons::BATTERY_FULL, dim)
                    .rotation(Rotation::Solid(Radians(-std::f32::consts::FRAC_PI_2))),
                text(format!("{}%", battery.percent))
                    .size(tok::FS_CAPTION)
                    .font(mono(Weight::Semibold))
                    .color(c(tok::TEXT_PRIMARY)),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
    }
    let status = container(status).align_right(Length::Fill);

    // Unread notifications: bell with a red dot at its top-right corner.
    let dot = container(Space::new().width(6).height(6)).style(|_| container::Style {
        background: Some(Background::Color(c(tok::DANGER))),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });
    let bell = stack![
        icon(icons::BELL, c(tok::ACCENT_TEXT)),
        container(dot)
            .width(14)
            .height(14)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Top),
    ];
    // As in the design component, the bell only appears when something is unread.
    let mut centre = row![].spacing(8).align_y(Alignment::Center);
    if panel.unread > 0 {
        centre = centre.push(bell);
    }
    let centre = centre.push(clock);

    let bar = container(row![left, centre, status].align_y(Alignment::Center))
        .padding([0, 12])
        .height(Length::Fill)
        .width(Length::Fill)
        .align_y(Alignment::Center);
    let rule = container(Space::new().height(1).width(Length::Fill)).style(|_| container::Style {
        background: Some(Background::Color(c(tok::BORDER_SUBTLE))),
        ..container::Style::default()
    });

    container(column![bar, rule])
        .style(|_| container::Style {
            background: Some(Background::Color(c(tok::SURFACE_PANEL))),
            ..container::Style::default()
        })
        .into()
}

fn main() -> Result<(), iced_layershell::Error> {
    application(Panel::default, namespace, update, view)
        .style(|_, _| iced::theme::Style {
            background_color: IcedColor::TRANSPARENT,
            text_color: c(tok::TEXT_PRIMARY),
        })
        .subscription(subscription)
        .settings(Settings {
            fonts: vec![SANS.into(), MONO.into()],
            default_font: sans(Weight::Normal),
            default_text_size: tok::FS_BODY.into(),
            antialiasing: true,
            layer_settings: LayerShellSettings {
                size: Some((0, tok::H_PANEL as u32)),
                exclusive_zone: tok::H_PANEL as i32,
                anchor: Anchor::Top | Anchor::Left | Anchor::Right,
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::None,
                start_mode: StartMode::Active,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
