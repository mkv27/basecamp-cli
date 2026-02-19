use crate::error::{AppError, AppResult};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

const SUCCESS_BODY: &str =
    "<html><body><h1>Basecamp login complete</h1><p>You can close this window.</p></body></html>";
const FAILURE_BODY: &str = "<html><body><h1>Basecamp login failed</h1><p>You can return to the terminal and retry.</p></body></html>";

#[derive(Debug)]
pub struct CallbackPayload {
    pub code: String,
    pub state: String,
}

pub struct CallbackServer {
    listener: TcpListener,
    expected_path: String,
    timeout: Duration,
}

impl CallbackServer {
    pub fn bind(redirect_uri: &str, timeout: Duration) -> AppResult<Self> {
        let parsed = Url::parse(redirect_uri)
            .map_err(|err| AppError::invalid_input(format!("Invalid redirect_uri: {err}")))?;

        if parsed.scheme() != "http" {
            return Err(AppError::invalid_input(
                "redirect_uri for CLI login must use http loopback (for example http://127.0.0.1:45455/callback).",
            ));
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| AppError::invalid_input("redirect_uri must include a host."))?;

        if host != "127.0.0.1" && host != "localhost" {
            return Err(AppError::invalid_input(
                "redirect_uri host must be localhost or 127.0.0.1 for CLI login.",
            ));
        }

        let port = parsed.port().ok_or_else(|| {
            AppError::invalid_input(
                "redirect_uri must include an explicit port for local callback handling.",
            )
        })?;

        let expected_path = if parsed.path().is_empty() {
            "/".to_string()
        } else {
            parsed.path().to_string()
        };

        let bind_addr = format!("127.0.0.1:{port}");
        let listener = TcpListener::bind(&bind_addr).map_err(|err| {
            AppError::oauth(format!(
                "Failed to bind callback server on {bind_addr}: {err}"
            ))
        })?;

        listener.set_nonblocking(true).map_err(|err| {
            AppError::oauth(format!("Failed to configure callback server: {err}"))
        })?;

        Ok(Self {
            listener,
            expected_path,
            timeout,
        })
    }

    pub fn wait_for_code(self) -> AppResult<CallbackPayload> {
        let deadline = Instant::now() + self.timeout;

        while Instant::now() < deadline {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => {
                    return parse_callback_request(&mut stream, &self.expected_path);
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(err) => {
                    return Err(AppError::oauth(format!(
                        "Failed to receive callback request: {err}"
                    )));
                }
            }
        }

        Err(AppError::oauth(
            "Timed out waiting for OAuth callback. Try login again.",
        ))
    }
}

fn parse_callback_request(
    stream: &mut TcpStream,
    expected_path: &str,
) -> AppResult<CallbackPayload> {
    let mut buffer = [0_u8; 8192];
    let bytes_read = stream
        .read(&mut buffer)
        .map_err(|err| AppError::oauth(format!("Failed to read callback request: {err}")))?;

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| AppError::oauth("Received malformed callback request."))?;

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();

    if method != "GET" {
        write_response(stream, "405 Method Not Allowed", FAILURE_BODY)?;
        return Err(AppError::oauth(
            "Callback request used unsupported HTTP method.",
        ));
    }

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };

    if path != expected_path {
        write_response(stream, "404 Not Found", FAILURE_BODY)?;
        return Err(AppError::oauth(format!(
            "Callback path mismatch. Expected {expected_path}, got {path}."
        )));
    }

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;

    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        if key == "code" {
            code = Some(value.to_string());
        } else if key == "state" {
            state = Some(value.to_string());
        }
    }

    let code = code.ok_or_else(|| {
        let _ = write_response(stream, "400 Bad Request", FAILURE_BODY);
        AppError::oauth("OAuth callback did not include code parameter.")
    })?;

    let state = state.ok_or_else(|| {
        let _ = write_response(stream, "400 Bad Request", FAILURE_BODY);
        AppError::oauth("OAuth callback did not include state parameter.")
    })?;

    write_response(stream, "200 OK", SUCCESS_BODY)?;

    Ok(CallbackPayload { code, state })
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) -> AppResult<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    stream
        .write_all(response.as_bytes())
        .map_err(|err| AppError::oauth(format!("Failed to write callback response: {err}")))?;
    stream
        .flush()
        .map_err(|err| AppError::oauth(format!("Failed to flush callback response: {err}")))?;

    Ok(())
}
