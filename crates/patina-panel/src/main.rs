//! Patina top panel: Milestone 0 toolkit spike.
//!
//! A 36 px layer-shell panel drawn with iced, styled only from `patina-tokens`.

mod motion;

use std::time::{Duration, Instant};

use iced::font::{Family, Weight};
use iced::widget::{button, column, container, row, text, Space};
use iced::{
    Alignment, Background, Border, Color as IcedColor, Element, Font, Length, Subscription, Task,
};
use iced_layershell::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use patina_tokens as tok;

use motion::CubicBezier;

const SANS: &[u8] =
    include_bytes!("../../../assets/fonts/plus-jakarta-sans/PlusJakartaSans-Variable.ttf");
const MONO: &[u8] =
    include_bytes!("../../../assets/fonts/jetbrains-mono/JetBrainsMono-Variable.ttf");

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
}

impl Default for Panel {
    fn default() -> Self {
        Self {
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
        Message::Workspace(_) | Message::Launcher => {}
        _ => unreachable!(),
    }
    Task::none()
}

fn subscription(panel: &Panel) -> Subscription<Message> {
    if panel.animating() {
        iced::time::every(Duration::from_millis(8)).map(Message::Frame)
    } else {
        Subscription::none()
    }
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
    // Placeholder for the Lucide `layout-grid` icon until icons are added.
    let launcher = button(
        text("::")
            .size(tok::FS_SMALL)
            .font(sans(Weight::Bold))
            .color(c(tok::TEXT_ON_ACCENT)),
    )
    .height(24)
    .padding([0, 12])
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
    let title = text("Terminal")
        .size(tok::FS_SMALL)
        .font(sans(Weight::Semibold))
        .color(c(tok::TEXT_PRIMARY));

    let left = row![launcher, workspaces, title]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let clock = text("Sat 17 Oct \u{b7} 09:41")
        .size(tok::FS_SMALL)
        .font(mono(Weight::Medium))
        .color(c(tok::TEXT_PRIMARY));
    let status = row![
        text("wifi  bt  sun  vol")
            .size(tok::FS_CAPTION)
            .font(sans(Weight::Medium))
            .color(c(tok::TEXT_SECONDARY)),
        text("87%")
            .size(tok::FS_CAPTION)
            .font(mono(Weight::Semibold))
            .color(c(tok::TEXT_PRIMARY)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let status = container(status).align_right(Length::Fill);

    let bar = container(row![left, clock, status].align_y(Alignment::Center))
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
