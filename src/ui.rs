use dialoguer::console::{Color, Style};
use dialoguer::theme::ColorfulTheme;

pub fn prompt_theme() -> ColorfulTheme {
    ColorfulTheme {
        prompt_style: Style::new().for_stderr().fg(Color::Color256(252)),
        hint_style: Style::new().for_stderr().fg(Color::Color256(245)),
        ..ColorfulTheme::default()
    }
}
