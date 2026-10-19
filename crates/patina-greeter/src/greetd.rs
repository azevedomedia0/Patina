//! Signing in through greetd's IPC (`$GREETD_SOCK`): create a session for the user, answer its
//! prompts (secret prompts with the password), and start the Patina session.

use std::os::unix::net::UnixStream;

use greetd_ipc::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};

/// Signs `user` in with `password` and asks greetd to start `cmd` for them. Returns the
/// message to show when it did not work.
pub fn sign_in(user: &str, password: &str, cmd: Vec<String>) -> Result<(), String> {
    let path = std::env::var("GREETD_SOCK").map_err(|_| "Not started by greetd".to_string())?;
    let mut stream = UnixStream::connect(path).map_err(|e| e.to_string())?;
    let send = |req: Request, stream: &mut UnixStream| -> Result<Response, String> {
        req.write_to(stream).map_err(|e| e.to_string())?;
        Response::read_from(stream).map_err(|e| e.to_string())
    };

    let mut reply = send(
        Request::CreateSession {
            username: user.to_string(),
        },
        &mut stream,
    )?;
    loop {
        match reply {
            Response::AuthMessage {
                auth_message_type, ..
            } => {
                let response = match auth_message_type {
                    AuthMessageType::Secret => Some(password.to_string()),
                    // A visible prompt asks for the user name again.
                    AuthMessageType::Visible => Some(user.to_string()),
                    AuthMessageType::Info | AuthMessageType::Error => None,
                };
                reply = send(Request::PostAuthMessageResponse { response }, &mut stream)?;
            }
            Response::Success => break,
            Response::Error {
                error_type,
                description,
            } => {
                let _ = send(Request::CancelSession, &mut stream);
                return Err(match error_type {
                    ErrorType::AuthError => "Wrong password".into(),
                    ErrorType::Error => description,
                });
            }
        }
    }
    match send(
        Request::StartSession {
            cmd,
            env: Vec::new(),
        },
        &mut stream,
    )? {
        Response::Success => Ok(()),
        Response::Error { description, .. } => {
            let _ = send(Request::CancelSession, &mut stream);
            Err(description)
        }
        Response::AuthMessage { .. } => Err("greetd asked for more".into()),
    }
}
