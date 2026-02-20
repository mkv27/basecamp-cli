use dialoguer::console::{Color, Style};
use dialoguer::theme::ColorfulTheme;

pub fn prompt_theme() -> ColorfulTheme {
    let mut theme = ColorfulTheme::default();
    theme.prompt_style = Style::new().for_stderr().fg(Color::Color256(252));
    theme.hint_style = Style::new().for_stderr().fg(Color::Color256(245));
    theme
}
