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
pub fn select_micro_arch_variant(
    release_name: &str,
    variants: Vec<Download>,
    quick_mode: bool,
) -> Result<Download> {
    if variants.is_empty() {
        return Err(anyhow!("No architecture variants available"));
    }

    // A single variant for the selected architecture needs no menu.
    if variants.len() == 1 {
        return Ok(variants.into_iter().next().unwrap());
    }

    if quick_mode && let Some(default) = architecture_variants::select_default_variant(&variants) {
        let name = architecture_variants::MicroArchVariants::from_file_name(&default.file_name)
            .map(|variant| variant.name())
            .unwrap_or("unknown");
        println!("Selected {name} by default");
        return Ok(default);
    }

    // Extract and sort variants using libprotonup
    let sorted_variants = architecture_variants::extract_march_variants(&variants);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_download(file_name: &str) -> Download {
        Download {
            file_name: file_name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_single_variant_skips_menu() {
        let variants = vec![mock_download(
            "proton-cachyos-11.0-20260703-slr-arm64.tar.xz",
        )];
        let result = select_micro_arch_variant("cachyos-11.0-20260703-slr", variants, false);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().file_name,
            "proton-cachyos-11.0-20260703-slr-arm64.tar.xz"
        );
    }

    #[test]
    fn test_single_x86_variant_skips_menu() {
        let variants = vec![mock_download(
            "proton-cachyos-11.0-20260703-slr-x86_64.tar.xz",
        )];
        let result = select_micro_arch_variant("cachyos-11.0-20260703-slr", variants, false);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().file_name,
            "proton-cachyos-11.0-20260703-slr-x86_64.tar.xz"
        );
    }

    #[test]
    fn test_empty_variants_errors() {
        let result = select_micro_arch_variant("cachyos-11.0-20260703-slr", vec![], false);
        assert!(result.is_err());
    }
}
