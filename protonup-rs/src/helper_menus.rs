use inquire::{Confirm, InquireError, MultiSelect};
use libprotonup::apps::AppInstallations;

pub(crate) fn tag_menu(message: &str, options: Vec<String>) -> Result<Vec<String>, InquireError> {
    MultiSelect::new(message, options)
        .with_default(&[0_usize])
        .prompt()
}

pub(crate) fn variants_menu(
    message: &str,
    options: Vec<AppInstallations>,
) -> Result<Vec<AppInstallations>, InquireError> {
    MultiSelect::new(message, options)
        .with_default(&[0_usize])
        .prompt()
}

pub(crate) fn confirm_menu(text: String, help_text: String, default: bool) -> bool {
    let answer = Confirm::new(&text)
        .with_default(default)
        .with_help_message(&help_text)
        .prompt();

    answer.unwrap_or(false)
}
