//! Patina greeter: the prototype's sign-in screen for greetd. It runs inside
//! `patina-compositor --greeter patina-greeter` (see `packaging/common/greetd-config.toml`),
//! lists the people who can sign in, checks the password through greetd and starts
//! `patina-session` for them. The power button shuts the computer down.

mod greetd;

use std::time::Duration;

use iced::{futures::SinkExt, Color, Element, Subscription, Task};
use iced_layershell::{
    application,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings, StartMode},
    to_layer_message,
};
use patina_signin::{Action, Mode, Msg, SignIn};
use patina_tokens as tok;
use patina_ui::{self as ui, c};

/// What greetd starts after a successful sign-in.
fn session_command() -> Vec<String> {
    std::env::var("PATINA_GREETER_SESSION")
        .map(|s| s.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_else(|_| vec!["patina-session".into()])
}

struct Greeter {
    screen: SignIn,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    SignIn(Msg),
    Done(Result<(), String>),
    PowerFailed(String),
}

fn focus_password() -> Task<Message> {
    iced::widget::operation::focus(iced::widget::Id::new(patina_signin::PASSWORD_ID))
}

fn new() -> (Greeter, Task<Message>) {
    let users = patina_signin::users::all();
    let screen = SignIn::new(Mode::Greeter, users, 0, false);
    (Greeter { screen }, focus_password())
}

fn namespace() -> String {
    "patina-greeter".into()
}

fn update(g: &mut Greeter, message: Message) -> Task<Message> {
    match message {
        Message::SignIn(msg) => match g.screen.update(msg) {
            Some(Action::Authenticate { user, password }) => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        greetd::sign_in(&user, &password, session_command())
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                Message::Done,
            ),
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
            // Picking a user in the greeter just selects them.
            Some(Action::SwitchUser(_)) | None => Task::none(),
        },
        // greetd starts the session once this greeter (and its compositor) has gone.
        Message::Done(Ok(())) => std::process::exit(0),
        Message::Done(Err(e)) => {
            g.screen.failed(&e);
            focus_password()
        }
        Message::PowerFailed(e) => {
            eprintln!("patina-greeter: power off refused: {e}");
            g.screen.ending = None;
            g.screen.failed("Shut down was refused");
            Task::none()
        }
        _ => Task::none(),
    }
}

fn view(g: &Greeter) -> Element<'_, Message> {
    g.screen.view().map(Message::SignIn)
}

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

fn subscription(_: &Greeter) -> Subscription<Message> {
    Subscription::run(minutes)
}

fn main() -> Result<(), iced_layershell::Error> {
    application(new, namespace, update, view)
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
            layer_settings: LayerShellSettings {
                // The whole screen, above everything, with the keyboard.
                size: Some((0, 0)),
                exclusive_zone: -1,
                anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                layer: Layer::Overlay,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                start_mode: StartMode::Active,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
