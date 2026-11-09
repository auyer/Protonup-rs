//! TUI-specific tool variant selection menu.
//!
//! This module provides the TUI selection menu using `inquire`.
//! The core variant detection logic is in `libprotonup::tool_variants`.

use anyhow::{Result, anyhow};
use inquire::Select;

use libprotonup::downloads::Download;
use libprotonup::tool_variants;

/// Menu for selecting a tool variant (micro-architecture, WoW64, native, ...).
///
/// In quick mode, returns the `_v2` variant by default (or the first available).
/// In interactive mode, shows a selection menu listing every offered variant
/// with its description followed by the archive file name.
pub fn select_tool_variant(
    release_name: &str,
    variants: Vec<Download>,
    quick_mode: bool,
) -> Result<Download> {
    if variants.is_empty() {
        return Err(anyhow!("No asset variants available"));
    }

    // A single variant for the selected architecture needs no menu.
    if variants.len() == 1 {
        return Ok(variants.into_iter().next().unwrap());
    }

    if quick_mode && let Some(default) = tool_variants::select_default_variant(&variants) {
        let label = tool_variants::ToolVariantChoice::from_file_name(&default.file_name).label();
        println!("Selected {label} by default");
        return Ok(default);
    }

    // Sort variants using libprotonup: known flavours first, unknown ones as-is.
    let sorted_variants = tool_variants::extract_tool_variants(&variants);

    if sorted_variants.is_empty() {
        return Ok(variants.into_iter().next().unwrap());
    }

    let selected = Select::new(
        format!("Select a variant for release '{}':", release_name).as_str(),
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
        let result = select_tool_variant("cachyos-11.0-20260703-slr", variants, false);
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
        let result = select_tool_variant("cachyos-11.0-20260703-slr", variants, false);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().file_name,
            "proton-cachyos-11.0-20260703-slr-x86_64.tar.xz"
        );
    }

    #[test]
    fn test_empty_variants_errors() {
        let result = select_tool_variant("cachyos-11.0-20260703-slr", vec![], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_quick_mode_picks_v2() {
        let variants = vec![
            mock_download("proton-cachyos-11.0-x86_64.tar.xz"),
            mock_download("proton-cachyos-11.0-x86_64_v2.tar.xz"),
            mock_download("proton-cachyos-11.0-x86_64_v3.tar.xz"),
        ];
        let result = select_tool_variant("cachyos-11.0", variants, true).unwrap();
        assert!(result.file_name.contains("_v2"));
    }

    #[test]
    fn test_menu_labels_include_file_name() {
        // Verifies the rendering contract used by the menu: known flavours get
        // their description plus the file name; unknown ones show the file name.
        let variants = vec![
            mock_download("proton-wineland-11.0-20261106-x86_64.tar.xz"),
            mock_download("proton-wineland-11.0-20261106-x86_64_wow64.tar.xz"),
        ];
        let sorted = tool_variants::extract_tool_variants(&variants);
        let rendered: Vec<String> = sorted.iter().map(|v| v.to_string()).collect();

        assert_eq!(
            rendered,
            vec![
                "x86_64 - Universal - all x86-64 CPUs (proton-wineland-11.0-20261106-x86_64.tar.xz)"
                    .to_string(),
                "WoW64 - 64-bit only (proton-wineland-11.0-20261106-x86_64_wow64.tar.xz)"
                    .to_string(),
            ]
        );
    }
}
