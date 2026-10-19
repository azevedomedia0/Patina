//! The shared look of the Patina shell surfaces, built with iced: fonts, token colours, Lucide
//! icons, and the design system's controls (switch, slider, icon button, ghost button, menu
//! rows, app icons, glass and card containers). Everything is taken from `patina-tokens` and the
//! design system components in the handoff (`_ds/.../components`).
//!
//! Hover and press transitions in the components are not animated (decision of 2026-10-19:
//! only the dock's hover scale moves).

use iced::{
    border::Radius,
    font::{Family, Weight},
    widget::{button, container, image, row, slider as islider, svg, text, Space},
    Alignment, Background, Border, Color, Element, Font, Length, Shadow, Vector,
};
use patina_tokens as tok;

pub mod motion;

pub const SANS_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/plus-jakarta-sans/PlusJakartaSans-Variable.ttf");
pub const MONO_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/jetbrains-mono/JetBrainsMono-Variable.ttf");

pub fn sans(weight: Weight) -> Font {
    Font {
        family: Family::Name("Plus Jakarta Sans"),
        weight,
        ..Font::DEFAULT
    }
}

pub fn mono(weight: Weight) -> Font {
    Font {
        family: Family::Name("JetBrains Mono"),
        weight,
        ..Font::DEFAULT
    }
}

/// A design token colour as an iced colour.
pub fn c(t: tok::Color) -> Color {
    Color::from_rgba8(t.r, t.g, t.b, t.a)
}

/// `color` at `alpha` times its opacity: the design's disabled controls are at 40%.
pub fn faded(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

/// Opacity of disabled controls (`opacity: .4` in the design's Button).
pub const DISABLED: f32 = 0.4;

pub fn lerp(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

macro_rules! icons {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// Lucide icons (ISC licence, `assets/icons/lucide/LICENSE`), stroke 2, 24 px grid.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Icon { $($variant),* }

        impl Icon {
            pub fn bytes(self) -> &'static [u8] {
                match self {
                    $(Icon::$variant => include_bytes!(concat!("../../../assets/icons/lucide/", $file, ".svg"))),*
                }
            }
        }
    };
}

icons! {
    BatteryFull => "battery-full", Bell => "bell", BellOff => "bell-off",
    Bluetooth => "bluetooth", BluetoothOff => "bluetooth-off", Cast => "cast", Check => "check",
    ChevronDown => "chevron-down", ChevronRight => "chevron-right", ExternalLink => "external-link",
    FolderPlus => "folder-plus", Gamepad => "gamepad-2", Headphones => "headphones",
    Image => "image", Keyboard => "keyboard", Laptop => "laptop", LayoutGrid => "layout-grid",
    Lock => "lock", LogOut => "log-out", Monitor => "monitor", Mouse => "mouse", Pin => "pin",
    Power => "power", Printer => "printer", Projector => "projector", RotateCw => "rotate-cw",
    Search => "search", Settings => "settings", Shield => "shield", Smartphone => "smartphone",
    Speaker => "speaker", Sun => "sun", Tv => "tv", Volume2 => "volume-2", VolumeX => "volume-x",
    Wifi => "wifi", WifiOff => "wifi-off", X => "x",
}

/// A tinted `size` px icon.
pub fn icon(which: Icon, size: f32, color: Color) -> svg::Svg<'static> {
    svg(svg::Handle::from_memory(which.bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| svg::Style { color: Some(color) })
}

pub fn label<'a>(
    s: impl text::IntoFragment<'a>,
    size: f32,
    weight: Weight,
    color: Color,
) -> text::Text<'a> {
    text(s).size(size).font(sans(weight)).color(color)
}

pub fn mono_label<'a>(s: impl text::IntoFragment<'a>, size: f32, color: Color) -> text::Text<'a> {
    text(s).size(size).font(mono(Weight::Medium)).color(color)
}

fn radius(r: f32) -> Radius {
    r.into()
}

/// `--shadow-popover`: `0 16px 32px -8px rgba(0,0,0,.55)`. Used only where iced draws the
/// shadow itself; for layer surfaces the compositor draws it.
pub fn popover_shadow() -> Shadow {
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
        offset: Vector::new(0.0, 16.0),
        blur_radius: 32.0,
    }
}

/// Popovers and menus: translucent ink (`--surface-window`); the compositor blurs behind it.
pub fn glass(r: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(c(tok::SURFACE_WINDOW))),
        border: Border {
            radius: radius(r),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The dock and panel: `--surface-panel` with a subtle 1 px border.
pub fn panel_glass(r: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(c(tok::SURFACE_PANEL))),
        border: Border {
            radius: radius(r),
            width: 1.0,
            color: c(tok::BORDER_SUBTLE),
        },
        ..container::Style::default()
    }
}

/// A notification card or a tile: `--surface-card`, `--radius-lg`.
pub fn card(_: &iced::Theme) -> container::Style {
    filled(c(tok::SURFACE_CARD), tok::RADIUS_LG)(&iced::Theme::Dark)
}

/// The darker inset behind slider groups and device lists.
pub fn inset(_: &iced::Theme) -> container::Style {
    filled(c(tok::INSET), tok::RADIUS_LG)(&iced::Theme::Dark)
}

pub fn filled(color: Color, r: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: radius(r),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A 1 px `--border-subtle` rule.
pub fn rule<'a, M: 'a>(horizontal: bool) -> Element<'a, M> {
    let space = if horizontal {
        Space::new().width(Length::Fill).height(1)
    } else {
        Space::new().width(1).height(Length::Fill)
    };
    container(space)
        .style(|_| container::Style {
            background: Some(Background::Color(c(tok::BORDER_SUBTLE))),
            ..container::Style::default()
        })
        .into()
}

/// Background for a hoverable row or button: transparent, `--surface-hover` on hover.
pub fn hover_bg(status: button::Status, base: Option<Color>) -> Option<Background> {
    match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(Background::Color(c(tok::SURFACE_HOVER)))
        }
        _ => base.map(Background::Color),
    }
}

/// The design's Switch: a 36 x 20 pill, accent when on, with a 16 px white knob.
pub fn switch<'a, M: Clone + 'a>(checked: bool, on_toggle: Option<M>) -> Element<'a, M> {
    let alpha = if on_toggle.is_some() { 1.0 } else { DISABLED };
    let knob = container(Space::new().width(16).height(16)).style(move |_| container::Style {
        background: Some(Background::Color(faded(Color::WHITE, alpha))),
        border: Border {
            radius: radius(8.0),
            ..Border::default()
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35 * alpha),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        },
        ..container::Style::default()
    });
    let track = if checked {
        c(tok::ACCENT)
    } else {
        c(tok::TRACK)
    };
    let inner = container(knob)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(if checked {
            Alignment::End
        } else {
            Alignment::Start
        })
        .align_y(Alignment::Center);
    let mut b = button(inner)
        .width(36)
        .height(20)
        .padding(2)
        .style(move |_, _| button::Style {
            background: Some(Background::Color(faded(track, alpha))),
            border: Border {
                radius: radius(tok::RADIUS_PILL),
                ..Border::default()
            },
            ..button::Style::default()
        });
    if let Some(m) = on_toggle {
        b = b.on_press(m);
    }
    b.into()
}

/// The design's Slider: a 4 px rail, accent up to the value, `--track` after.
pub fn slider<'a, M: Clone + 'a>(
    value: u8,
    on_change: Option<fn(u8) -> M>,
    on_release: Option<M>,
) -> Element<'a, M> {
    let enabled = on_change.is_some();
    let alpha = if enabled { 1.0 } else { DISABLED };
    let style = move |_: &iced::Theme, _| islider::Style {
        rail: islider::Rail {
            backgrounds: (
                Background::Color(faded(c(tok::ACCENT), alpha)),
                Background::Color(faded(c(tok::TRACK), alpha)),
            ),
            width: 4.0,
            border: Border {
                radius: radius(2.0),
                ..Border::default()
            },
        },
        handle: islider::Handle {
            shape: islider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(faded(c(tok::ACCENT), alpha)),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    };
    match on_change {
        Some(f) => {
            let mut s = islider(0..=100u8, value, f).height(16).style(style);
            if let Some(r) = on_release {
                s = s.on_release(r);
            }
            s.into()
        }
        // A slider without a message cannot move: draw it with a no-op target.
        None => container(
            iced::widget::progress_bar(0.0..=100.0, f32::from(value))
                .girth(4)
                .style(move |_| iced::widget::progress_bar::Style {
                    background: Background::Color(faded(c(tok::TRACK), alpha)),
                    bar: Background::Color(faded(c(tok::ACCENT), alpha)),
                    border: Border {
                        radius: radius(2.0),
                        ..Border::default()
                    },
                }),
        )
        .height(16)
        .align_y(Alignment::Center)
        .into(),
    }
}

/// A 28 px round icon button with a 16 px icon (quick settings footer).
pub fn round_icon_button<'a, M: Clone + 'a>(which: Icon, on_press: Option<M>) -> Element<'a, M> {
    let enabled = on_press.is_some();
    let alpha = if enabled { 1.0 } else { DISABLED };
    let b = button(
        container(
            svg(svg::Handle::from_memory(which.bytes()))
                .width(16)
                .height(16)
                .style(move |_, status| svg::Style {
                    color: Some(faded(
                        if enabled && matches!(status, svg::Status::Hovered) {
                            c(tok::TEXT_PRIMARY)
                        } else {
                            c(tok::TEXT_SECONDARY)
                        },
                        alpha,
                    )),
                }),
        )
        .center(Length::Fill),
    )
    .width(28)
    .height(28)
    .padding(0)
    .style(move |_, status| button::Style {
        background: if enabled {
            hover_bg(status, None)
        } else {
            None
        },
        border: Border {
            radius: radius(14.0),
            ..Border::default()
        },
        ..button::Style::default()
    });
    match on_press {
        Some(m) => b.on_press(m).into(),
        None => b.into(),
    }
}

/// The design's ghost Button, small: 24 px pill, 12 px semibold, secondary text.
pub fn ghost_button<'a, M: Clone + 'a>(text_: &'a str, on_press: Option<M>) -> Element<'a, M> {
    let enabled = on_press.is_some();
    let b = button(
        container(text(text_).size(tok::FS_SMALL).font(sans(Weight::Semibold)))
            .center_y(Length::Fill),
    )
    .height(24)
    .padding([0, 10])
    .style(move |_, status| {
        let hovered =
            enabled && matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(c(tok::SURFACE_HOVER)))
            } else {
                None
            },
            text_color: faded(
                if hovered {
                    c(tok::TEXT_PRIMARY)
                } else {
                    c(tok::TEXT_SECONDARY)
                },
                if enabled { 1.0 } else { DISABLED },
            ),
            border: Border {
                radius: radius(tok::RADIUS_PILL),
                ..Border::default()
            },
            ..button::Style::default()
        }
    });
    match on_press {
        Some(m) => b.on_press(m).into(),
        None => b.into(),
    }
}

/// One item of a context menu: 28 px high, 15 px icon, 13 px medium label, an optional
/// shortcut in mono on the right; red when it destroys something.
pub fn menu_row<'a, M: Clone + 'a>(
    which: Icon,
    label_: String,
    key: &'a str,
    danger: bool,
    on_press: Option<M>,
) -> Element<'a, M> {
    let enabled = on_press.is_some();
    let alpha = if enabled { 1.0 } else { DISABLED };
    let color = faded(
        if danger {
            c(tok::DANGER)
        } else {
            c(tok::TEXT_PRIMARY)
        },
        alpha,
    );
    let content = row![
        icon(which, 15.0, color),
        container(
            text(label_)
                .size(13)
                .font(sans(Weight::Medium))
                .color(color)
        )
        .width(Length::Fill),
        text(key)
            .size(10)
            .font(mono(Weight::Medium))
            .color(faded(c(tok::TEXT_MUTED), alpha)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let b = button(container(content).center_y(Length::Fill))
        .width(Length::Fill)
        .height(28)
        .padding([0, 8])
        .style(move |_, status| button::Style {
            background: if enabled {
                hover_bg(status, None)
            } else {
                None
            },
            border: Border {
                radius: radius(tok::RADIUS_MD),
                ..Border::default()
            },
            ..button::Style::default()
        });
    match on_press {
        Some(m) => b.on_press(m).into(),
        None => b.into(),
    }
}

/// The line between groups of menu items: 1 px, 4 px above and below, 6 px in from the sides.
pub fn menu_separator<'a, M: 'a>() -> Element<'a, M> {
    container(rule(true)).padding([4, 6]).into()
}

/// An app icon image from `patina-apps`.
pub fn rgba_handle(icon: &patina_apps::icons::Rgba) -> image::Handle {
    image::Handle::from_rgba(icon.width, icon.height, icon.pixels.clone())
}

/// An app icon: a squircle-ish image with `--radius-icon` (22%) corners, `size` px, drawn at
/// `scale` times that (the dock's hover grows icons to 1.08).
pub fn app_icon<'a, M: 'a>(handle: &image::Handle, size: f32, scale: f32) -> Element<'a, M> {
    image(handle.clone())
        .width(size)
        .height(size)
        .scale(scale)
        .border_radius(size * 0.22)
        .into()
}

/// Lays out a fixed-height text line so heights stay predictable (popovers size their
/// surfaces from their content).
pub fn line<'a, M: 'a>(t: text::Text<'a>, height: f32) -> Element<'a, M> {
    container(t).height(height).center_y(height).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_an_svg() {
        for i in [
            Icon::Wifi,
            Icon::Cast,
            Icon::X,
            Icon::Power,
            Icon::ChevronDown,
        ] {
            assert!(std::str::from_utf8(i.bytes()).unwrap().contains("<svg"));
        }
    }

    #[test]
    fn token_colours_convert() {
        let w = c(tok::SURFACE_WINDOW);
        assert!((w.a - 0.78).abs() < 0.01);
        assert_eq!(faded(Color::WHITE, DISABLED).a, 0.4);
    }
}
