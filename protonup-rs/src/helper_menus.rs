use inquire::{Confirm, InquireError, MultiSelect};

/// Creates a inquire::MultiSelect menu with the first option selected
pub(crate) fn multiple_select_menu<T>(
    message: &str,
    options: Vec<T>,
) -> Result<Vec<T>, InquireError>
where
    T: std::fmt::Display,
{
    MultiSelect::new(message, options)
        .with_default(&[0])
        .prompt()
}

pub(crate) fn confirm_menu(text: String, help_text: String, default: bool) -> bool {
    let answer = Confirm::new(&text)
        .with_default(default)
        .with_help_message(&help_text)
        .prompt();

    answer.unwrap_or(false)
}
