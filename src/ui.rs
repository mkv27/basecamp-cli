use crate::error::AppError;
use inquire::error::InquireError;
use inquire::ui::{Color, RenderConfig, StyleSheet};

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
            AppError::invalid_input(format!("{action} cancelled."))
        }
        _ => AppError::invalid_input(format!("Failed to {action}: {err}")),
    }
}
