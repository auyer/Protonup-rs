//! TUI-specific architecture variant selection menu.
//!
//! This module provides the TUI selection menu using `inquire`.
//! The core variant detection logic is in `libprotonup::architecture_variants`.

use anyhow::{Result, anyhow};
use inquire::Select;

use libprotonup::architecture_variants;
use libprotonup::downloads::Download;

/// Menu for selecting proton cachyos architecture variant.
///
/// In quick mode, returns the `_v2` variant by default (or first available).
/// In interactive mode, shows a selection menu with descriptions.
pub fn select_architecture_variant(
    release_name: &str,
    variants: Vec<Download>,
    quick_mode: bool,
) -> Result<Download> {
    if variants.is_empty() {
        return Err(anyhow!("No architecture variants available"));
    }

    if quick_mode && let Some(default) = architecture_variants::select_default_variant(&variants) {
        println!(
            "Selected {} by default",
            architecture_variants::get_variant_name(
                architecture_variants::get_architecture_variant(&default.file_name)
            )
        );
        return Ok(default);
    }

    // Extract and sort variants using libprotonup
    let sorted_variants = architecture_variants::extract_variants(&variants);

    if sorted_variants.is_empty() {
        return Ok(variants.into_iter().next().unwrap());
    }

    let selected = Select::new(
        format!("Select CPU architecture for release '{}':", release_name).as_str(),
        sorted_variants,
    )
    .prompt()
    .unwrap_or_else(|_| std::process::exit(0));

    Ok(selected.download)
}
