use crate::error::AppError;
use inquire::error::InquireError;
use inquire::ui::{Color, RenderConfig, StyleSheet};
use std::io::{self, IsTerminal, Write};

pub fn configure_prompt_rendering() {
    let render_config = RenderConfig {
        prompt: StyleSheet::new().with_fg(Color::AnsiValue(252)),
        default_value: StyleSheet::new().with_fg(Color::DarkGrey),
        placeholder: StyleSheet::new().with_fg(Color::DarkGrey),
        help_message: StyleSheet::new().with_fg(Color::DarkGrey),
        ..RenderConfig::default()
    };
    inquire::set_global_render_config(render_config);
}

pub fn prompt_error(action: &str, err: InquireError) -> AppError {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            clear_active_terminal_line();
            AppError::invalid_input(format!("{action} cancelled."))
        }
        _ => AppError::invalid_input(format!("Failed to {action}: {err}")),
    }
}

fn clear_active_terminal_line() {
    if !io::stderr().is_terminal() {
        return;
    }

    let mut stderr = io::stderr();
    let _ = stderr.write_all(b"\r\x1b[2K");
    let _ = stderr.flush();
}
