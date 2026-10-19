//! The sign-in screen from the prototype, shared by the lock screen (`patina-lock`) and the
//! greeter (`patina-greeter`): the wallpaper, a 128 px clock with the date under it, the user's
//! avatar (initials on a circle) and name, a pill password field with a round arrow button, an
//! error line, a "Switch user" button bottom-left with its popover, and a power button
//! bottom-right. Shutting down shows the dark "Shutting down…" screen.
//!
//! The screen does not authenticate: [`SignIn::update`] returns an [`Action`] for the program
//! to carry out (PAM for the lock screen, greetd for the greeter).

pub mod switch;
pub mod users;

use iced::{
    font::Weight,
    widget::{button, column, container, image, row, stack, text, text_input, Space},
    Alignment, Background, Border, Color, ContentFit, Element, Length, Padding,
};
use patina_tokens as tok;
use patina_ui::{self as ui, c, Icon};

pub use users::User;

/// The password field, for giving it focus.
pub const PASSWORD_ID: &str = "patina-password";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The lock screen: the signed-in user, who unlocks.
    Lock,
    /// The greeter: any user may be picked to sign in.
    Greeter,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Password(String),
    Submit,
    ToggleSwitch,
    CloseSwitch,
    PickUser(usize),
    PowerOff,
    /// The minute changed.
    Tick,
}

/// What the program should do.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Check `password` for `user`.
    Authenticate {
        user: String,
        password: String,
    },
    /// Lock screen: hand over to the display manager to sign in as this user.
    SwitchUser(String),
    PowerOff,
}

pub struct SignIn {
    pub mode: Mode,
    pub users: Vec<User>,
    pub selected: usize,
    pub password: String,
    /// Shown in red under the field; empty for none.
    pub error: String,
    pub switch_open: bool,
    /// Lock screen: whether the display manager can switch to another user.
    pub can_switch: bool,
    /// Waiting for an answer: input is ignored.
    pub busy: bool,
    /// The dark end-of-session screen with this message ("Shutting down…").
    pub ending: Option<String>,
    clock: String,
    date: String,
    wallpaper: image::Handle,
}

impl SignIn {
    pub fn new(mode: Mode, users: Vec<User>, selected: usize, can_switch: bool) -> Self {
        let (clock, date) = now();
        Self {
            mode,
            users,
            selected,
            password: String::new(),
            error: String::new(),
            switch_open: false,
            can_switch,
            busy: false,
            ending: None,
            clock,
            date,
            wallpaper: wallpaper(),
        }
    }

    pub fn user(&self) -> Option<&User> {
        self.users.get(self.selected)
    }

    fn can_pick(&self, i: usize) -> bool {
        i == self.selected || self.mode == Mode::Greeter || self.can_switch
    }

    pub fn update(&mut self, msg: Msg) -> Option<Action> {
        match msg {
            Msg::Password(p) if !self.busy => {
                self.password = p;
                self.error.clear();
            }
            Msg::Submit if !self.busy => {
                if self.password.is_empty() {
                    // The prototype's message for an empty field.
                    self.error = "Enter your password".into();
                    return None;
                }
                let user = self.user()?.login.clone();
                self.busy = true;
                return Some(Action::Authenticate {
                    user,
                    password: self.password.clone(),
                });
            }
            Msg::ToggleSwitch => self.switch_open = !self.switch_open,
            Msg::CloseSwitch => self.switch_open = false,
            Msg::PickUser(i) => {
                self.switch_open = false;
                if i == self.selected || i >= self.users.len() || !self.can_pick(i) {
                    return None;
                }
                self.password.clear();
                self.error.clear();
                match self.mode {
                    Mode::Greeter => self.selected = i,
                    Mode::Lock => return Some(Action::SwitchUser(self.users[i].login.clone())),
                }
            }
            Msg::PowerOff => {
                self.ending = Some("Shutting down…".into());
                return Some(Action::PowerOff);
            }
            Msg::Tick => (self.clock, self.date) = now(),
            _ => {}
        }
        None
    }

    /// The answer came back negative: show `message` and let the user try again.
    pub fn failed(&mut self, message: &str) {
        self.busy = false;
        self.password.clear();
        self.error = message.into();
    }

    pub fn view(&self) -> Element<'_, Msg> {
        if let Some(message) = &self.ending {
            return ending(message);
        }
        let backdrop = image(self.wallpaper.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Cover);

        // Clock 128 px semibold, date 22 px secondary, 56 px from the top.
        let clock = column![
            text(&self.clock)
                .size(128)
                .font(ui::sans(Weight::Semibold))
                .color(c(tok::TEXT_PRIMARY)),
            text(&self.date)
                .size(22)
                .font(ui::sans(Weight::Normal))
                .color(c(tok::TEXT_SECONDARY)),
        ]
        .spacing(6)
        .align_x(Alignment::Center);
        let top = container(clock)
            .width(Length::Fill)
            .padding(Padding {
                top: 56.0,
                ..Padding::ZERO
            })
            .align_x(Alignment::Center);

        let user = self.user();
        let initials = user.map(|u| u.initials()).unwrap_or_default();
        let avatar = container(
            text(initials)
                .size(28)
                .font(ui::sans(Weight::Bold))
                .color(Color::WHITE),
        )
        .center(84)
        .style(circle(avatar_colour(self.selected)));
        let name = text(user.map(|u| u.display_name()).unwrap_or_default())
            .size(15)
            .font(ui::sans(Weight::Bold))
            .color(c(tok::TEXT_PRIMARY));

        let mut field = text_input("Password", &self.password)
            .id(PASSWORD_ID)
            .secure(true)
            .width(240)
            .padding(Padding::from([9, 16]))
            .size(13)
            .font(ui::sans(Weight::Normal))
            .style(|_, _| text_input::Style {
                background: Background::Color(c(tok::FIELD_BG)),
                border: Border {
                    radius: tok::RADIUS_PILL.into(),
                    ..Border::default()
                },
                icon: c(tok::TEXT_MUTED),
                placeholder: c(tok::TEXT_MUTED),
                value: c(tok::TEXT_PRIMARY),
                selection: c(tok::SURFACE_ACTIVE),
            });
        if !self.busy {
            field = field.on_input(Msg::Password).on_submit(Msg::Submit);
        }
        let arrow =
            button(container(ui::icon(Icon::ArrowRight, 18.0, Color::WHITE)).center(Length::Fill))
                .width(36)
                .height(36)
                .padding(0)
                .on_press_maybe((!self.busy).then_some(Msg::Submit))
                .style(|_, status| button::Style {
                    background: Some(Background::Color(match status {
                        button::Status::Hovered | button::Status::Pressed => c(tok::ACCENT_HOVER),
                        _ => c(tok::ACCENT),
                    })),
                    border: Border {
                        radius: 18.0.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                });
        let error = container(
            text(&self.error)
                .size(12)
                .font(ui::sans(Weight::Normal))
                .color(c(tok::DANGER)),
        )
        .height(16);

        let centre = column![
            avatar,
            container(name).padding(Padding {
                bottom: 12.0,
                ..Padding::ZERO
            }),
            row![field, arrow].spacing(8).align_y(Alignment::Center),
            error,
        ]
        .spacing(14)
        .align_x(Alignment::Center);
        let middle = container(centre).center(Length::Fill);

        // Switching does something in the greeter, or with a display manager that can.
        let switchable = self.users.len() > 1
            && (0..self.users.len()).any(|i| i != self.selected && self.can_pick(i));
        let mut bottom_left = column![].spacing(8).align_x(Alignment::Start);
        if self.switch_open {
            bottom_left = bottom_left.push(self.switch_popover());
        }
        bottom_left = bottom_left.push(round_button(
            Icon::Users,
            switchable.then_some(Msg::ToggleSwitch),
        ));
        let corners = container(
            row![
                container(bottom_left).align_bottom(Length::Fill),
                Space::new().width(Length::Fill),
                container(round_button(Icon::Power, Some(Msg::PowerOff)))
                    .align_bottom(Length::Fill),
            ]
            .height(Length::Fill),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        let content = stack![top, middle, corners]
            .width(Length::Fill)
            .height(Length::Fill);
        // A click on the background closes the popover, as in the prototype.
        iced::widget::mouse_area(
            stack![backdrop, content]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Msg::CloseSwitch)
        .into()
    }

    fn switch_popover(&self) -> Element<'_, Msg> {
        let mut list = column![container(
            text("SWITCH USER")
                .size(10)
                .font(ui::sans(Weight::Semibold))
                .color(c(tok::TEXT_MUTED))
        )
        .padding(Padding {
            top: 6.0,
            right: 8.0,
            bottom: 4.0,
            left: 8.0
        })]
        .spacing(2);
        for (i, u) in self.users.iter().enumerate() {
            let current = i == self.selected;
            let fg = if current {
                c(tok::ACCENT_TEXT)
            } else {
                c(tok::TEXT_PRIMARY)
            };
            let bg = current.then(|| c(tok::SURFACE_ACTIVE));
            let avatar = container(
                text(u.initials())
                    .size(12)
                    .font(ui::sans(Weight::Bold))
                    .color(Color::WHITE),
            )
            .center(30)
            .style(circle(avatar_colour(i)));
            let labels = column![
                text(u.display_name())
                    .size(13)
                    .font(ui::sans(Weight::Semibold))
                    .color(fg),
                text(u.sub_label())
                    .size(10)
                    .font(ui::sans(Weight::Normal))
                    .color(ui::faded(fg, 0.7)),
            ];
            let enabled = self.can_pick(i);
            let b = button(row![avatar, labels].spacing(10).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(44)
                .padding([0, 8])
                .on_press_maybe(enabled.then_some(Msg::PickUser(i)))
                .style(move |_, status| button::Style {
                    // `filter: brightness(1.15)` on hover.
                    background: match status {
                        button::Status::Hovered | button::Status::Pressed => {
                            Some(Background::Color(
                                bg.map(|b| ui::lerp(b, Color::WHITE, 0.08))
                                    .unwrap_or(c(tok::SURFACE_HOVER)),
                            ))
                        }
                        _ => bg.map(Background::Color),
                    },
                    text_color: if enabled {
                        fg
                    } else {
                        ui::faded(fg, ui::DISABLED)
                    },
                    border: Border {
                        radius: tok::RADIUS_MD.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                });
            list = list.push(b);
        }
        container(list)
            .width(220)
            .padding(6)
            .style(|_| container::Style {
                background: Some(Background::Color(c(tok::SURFACE_WINDOW))),
                border: Border {
                    radius: tok::RADIUS_XL.into(),
                    ..Border::default()
                },
                shadow: ui::popover_shadow(),
                ..container::Style::default()
            })
            .into()
    }
}

/// The first user's avatar is the accent colour, the others `--ink-500` (prototype `luBg`).
fn avatar_colour(i: usize) -> Color {
    if i == 0 {
        c(tok::ACCENT)
    } else {
        c(tok::INK_500)
    }
}

fn circle(bg: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 1000.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The 36 px round corner buttons: `--surface-card`, secondary icon; lighter on hover.
fn round_button<'a>(which: Icon, on_press: Option<Msg>) -> Element<'a, Msg> {
    let enabled = on_press.is_some();
    let alpha = if enabled { 1.0 } else { ui::DISABLED };
    button(
        container(
            iced::widget::svg(iced::widget::svg::Handle::from_memory(which.bytes()))
                .width(16)
                .height(16)
                .style(move |_, status| iced::widget::svg::Style {
                    color: Some(ui::faded(
                        if enabled && matches!(status, iced::widget::svg::Status::Hovered) {
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
    .width(36)
    .height(36)
    .padding(0)
    .on_press_maybe(on_press)
    .style(move |_, status| button::Style {
        background: Some(Background::Color(ui::faded(
            if enabled && matches!(status, button::Status::Hovered | button::Status::Pressed) {
                c(tok::SURFACE_HOVER)
            } else {
                c(tok::SURFACE_CARD)
            },
            alpha,
        ))),
        border: Border {
            radius: 18.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    })
    .into()
}

/// The dark screen while the session ends: `#05070b` with a 13 px message in `#8a93a3`.
pub fn ending<'a, M: 'a>(message: &'a str) -> Element<'a, M> {
    container(
        text(message)
            .size(13)
            .font(ui::sans(Weight::Normal))
            .color(Color::from_rgb8(0x8a, 0x93, 0xa3)),
    )
    .center(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgb8(0x05, 0x07, 0x0b))),
        ..container::Style::default()
    })
    .into()
}

/// "09:41" and "Monday 19 October" (the prototype's en-GB forms).
fn now() -> (String, String) {
    let t = chrono::Local::now();
    (
        t.format("%H:%M").to_string(),
        t.format("%A %-d %B").to_string(),
    )
}

/// The design's wallpaper, drawn once at a generous size and scaled to cover the screen.
fn wallpaper() -> image::Handle {
    let (w, h) = (1920, 1200);
    let bitmap = patina_frame::wallpaper::render(w, h);
    // BGRA (opaque) to RGBA.
    let mut rgba = bitmap.bgra;
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    image::Handle::from_rgba(w as u32, h as u32, rgba)
}

/// Asks logind to power off; returns why not if it refused.
pub fn power_off() -> Result<(), String> {
    logind_call("PowerOff")
}

pub fn reboot() -> Result<(), String> {
    logind_call("Reboot")
}

fn logind_call(method: &str) -> Result<(), String> {
    let conn = zbus::blocking::Connection::system().map_err(|e| e.to_string())?;
    let proxy = zbus::blocking::Proxy::new(
        &conn,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .map_err(|e| e.to_string())?;
    // `true`: polkit may ask for authorisation (when other users are signed in).
    proxy
        .call::<_, _, ()>(method, &(true,))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn users() -> Vec<User> {
        vec![
            User::new("ana", "Ana Lima", 1000),
            User::new("guest", "", 1001),
        ]
    }

    #[test]
    fn an_empty_password_asks_for_one() {
        let mut s = SignIn::new(Mode::Lock, users(), 0, false);
        assert_eq!(s.update(Msg::Submit), None);
        assert_eq!(s.error, "Enter your password");
    }

    #[test]
    fn submitting_asks_to_authenticate_once() {
        let mut s = SignIn::new(Mode::Lock, users(), 0, false);
        s.update(Msg::Password("secret".into()));
        assert_eq!(
            s.update(Msg::Submit),
            Some(Action::Authenticate {
                user: "ana".into(),
                password: "secret".into()
            })
        );
        assert_eq!(s.update(Msg::Submit), None, "busy until the answer");
        s.failed("Wrong password");
        assert!(!s.busy && s.password.is_empty());
        assert_eq!(s.error, "Wrong password");
    }

    #[test]
    fn the_greeter_selects_and_the_lock_screen_switches() {
        let mut g = SignIn::new(Mode::Greeter, users(), 0, false);
        assert_eq!(g.update(Msg::PickUser(1)), None);
        assert_eq!(g.selected, 1);

        let mut no_dm = SignIn::new(Mode::Lock, users(), 0, false);
        assert_eq!(no_dm.update(Msg::PickUser(1)), None);
        assert_eq!(no_dm.selected, 0, "nothing to switch with");

        let mut dm = SignIn::new(Mode::Lock, users(), 0, true);
        assert_eq!(
            dm.update(Msg::PickUser(1)),
            Some(Action::SwitchUser("guest".into()))
        );
    }

    #[test]
    fn power_off_shows_the_ending_screen() {
        let mut s = SignIn::new(Mode::Greeter, users(), 0, false);
        assert_eq!(s.update(Msg::PowerOff), Some(Action::PowerOff));
        assert_eq!(s.ending.as_deref(), Some("Shutting down…"));
    }
}
