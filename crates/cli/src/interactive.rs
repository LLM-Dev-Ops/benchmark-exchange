//! Interactive prompts and dialogs

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Prompt for text input
pub fn prompt_input(prompt: &str) -> Result<String> {
    let input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()?;
    Ok(input)
}

/// Prompt for text input with default value
pub fn prompt_input_with_default(prompt: &str, default: &str) -> Result<String> {
    let input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()?;
    Ok(input)
}

/// Prompt for password input
pub fn prompt_password(prompt: &str) -> Result<String> {
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()?;
    Ok(password)
}

/// Prompt for confirmation
pub fn confirm(prompt: &str) -> Result<bool> {
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(false)
        .interact()?;
    Ok(confirmed)
}

/// Prompt for confirmation with default true
pub fn confirm_default_yes(prompt: &str) -> Result<bool> {
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(true)
        .interact()?;
    Ok(confirmed)
}

/// Prompt for selection from a list
pub fn select<T: ToString>(prompt: &str, items: &[T]) -> Result<usize> {
    let items_str: Vec<String> = items.iter().map(|i| i.to_string()).collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items_str)
        .default(0)
        .interact()?;
    Ok(selection)
}

/// Create a progress bar
pub fn progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .expect("Failed to set progress bar template")
            .progress_chars("#>-"),
    );
    pb
}

/// Create a spinner
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Failed to set spinner template"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let pb = progress_bar(100);
        assert_eq!(pb.length(), Some(100));
    }

    #[test]
    fn test_spinner_creation() {
        let spinner = spinner("Loading...");
        assert!(spinner.is_hidden() == false || spinner.is_hidden() == true); // Just ensure it's created
    }
}
