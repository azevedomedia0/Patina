//! Patina lock screen: locks the session (`ext-session-lock-v1`) and shows the prototype's
//! sign-in screen for the signed-in user on every output until the right password is typed.
//! Passwords are checked with PAM (service `patina-lock`, `/etc/pam.d/patina-lock`).
//!
//! The compositor starts it on Ctrl+Alt+L, before the computer sleeps, after 15 idle minutes,
//! and on `loginctl lock-session`.

mod pam;

use std::time::Duration;

use iced::{futures::SinkExt, Color, Element, Subscription, Task};
use iced_sessionlock::{application, settings::Settings, to_session_message};
use patina_signin::{Action, Mode, Msg, SignIn};
use patina_tokens as tok;
use patina_ui::{self as ui, c};

const PAM_SERVICE: &str = "patina-lock";

struct Lock {
    screen: SignIn,
}

#[to_session_message]
#[derive(Debug, Clone)]
enum Message {
    SignIn(Msg),
    Checked(bool),
    PowerFailed(String),
}

fn in_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
        .unwrap_or(false)
}

/// Whether the display manager can show another user's sign-in while this session stays
/// locked: LightDM can (`dm-tool switch-to-user`).
fn can_switch() -> bool {
    std::env::var_os("XDG_SEAT_PATH").is_some() && in_path("dm-tool")
}

fn focus_password() -> Task<Message> {
    iced::widget::operation::focus(iced::widget::Id::new(patina_signin::PASSWORD_ID))
}

fn new() -> (Lock, Task<Message>) {
    let mut users = patina_signin::users::all();
    // The signed-in user first (present even when not a regular account).
    if let Some(me) = patina_signin::users::current() {
        users.retain(|u| u.login != me.login);
        users.insert(0, me);
    }
    let screen = SignIn::new(Mode::Lock, users, 0, can_switch());
    (Lock { screen }, focus_password())
}

fn update(lock: &mut Lock, message: Message) -> Task<Message> {
    match message {
        Message::SignIn(msg) => match lock.screen.update(msg) {
            Some(Action::Authenticate { user, password }) => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        pam::authenticate(PAM_SERVICE, &user, &password)
                    })
                    .await
                    .unwrap_or(false)
                },
                Message::Checked,
            ),
            Some(Action::SwitchUser(login)) => {
                // LightDM shows that user's sign-in on another terminal; this lock stays.
                if let Err(e) = std::process::Command::new("dm-tool")
                    .args(["switch-to-user", &login])
                    .spawn()
                {
                    eprintln!("patina-lock: cannot switch user: {e}");
                }
                Task::none()
            }
            Some(Action::PowerOff) => Task::perform(
                async {
                    tokio::task::spawn_blocking(patina_signin::power_off)
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                },
                |r| match r {
                    Ok(()) => Message::SignIn(Msg::Tick),
                    Err(e) => Message::PowerFailed(e),
                },
            ),
            None => Task::none(),
        },
        Message::Checked(true) => {
            // Unlock, then leave once the request has reached the compositor (the runtime
            // stops running tasks after unlocking, so a plain thread does it).
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(500));
                std::process::exit(0);
            });
            Task::done(Message::UnLock)
        }
        Message::Checked(false) => {
            lock.screen.failed("Wrong password");
            focus_password()
        }
        Message::PowerFailed(e) => {
            eprintln!("patina-lock: power off refused: {e}");
            lock.screen.ending = None;
            lock.screen.failed("Shut down was refused");
            Task::none()
        }
        Message::UnLock => Task::none(),
    }
}

fn view(lock: &Lock, _: iced::window::Id) -> Element<'_, Message> {
    lock.screen.view().map(Message::SignIn)
}

/// A message at the start of every minute, so the clock moves on the minute.
fn minutes() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, async |mut out| loop {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        tokio::time::sleep(Duration::from_secs(60 - secs % 60)).await;
        let _ = out.send(Message::SignIn(Msg::Tick)).await;
    })
}

fn subscription(_: &Lock) -> Subscription<Message> {
    Subscription::run(minutes)
}

fn main() -> Result<(), iced_sessionlock::Error> {
    application(new, update, view)
        .style(|_, _| iced::theme::Style {
            background_color: Color::BLACK,
            text_color: c(tok::TEXT_PRIMARY),
        })
        .subscription(subscription)
        .settings(Settings {
            fonts: vec![ui::SANS_BYTES.into(), ui::MONO_BYTES.into()],
            default_font: ui::sans(iced::font::Weight::Normal),
            default_text_size: tok::FS_BODY.into(),
            antialiasing: true,
            ..Default::default()
        })
        .run()
}
