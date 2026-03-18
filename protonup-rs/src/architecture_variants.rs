use anyhow::{Result, anyhow};
use inquire::Select;

use libprotonup::downloads::Download;

/// Architecture variant for Proton CachyOS
#[derive(Debug, Clone)]
pub struct ArchitectureVariant {
    /// The architecture variant name (x86_64, x86_64_v2, x86_64_v3, x86_64_v4...)
    pub name: String,
    /// Extended description of this variant
    pub description: String,
    /// The download information for this variant
    pub download: Download,
}

impl std::fmt::Display for ArchitectureVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.description)
    }
}

/// Extracts the architecture variant from file name
fn get_architecture_variant(file_name: &str) -> u8 {
    // 1: x86_64, 2: x86_64_v2, 3: x86_64_v3, 4: x86_64_v4
    if file_name.contains("_v4") {
        4
    } else if file_name.contains("_v3") {
        3
    } else if file_name.contains("_v2") {
        2
    } else if file_name.contains("-x86_64.") {
        1
    } else {
        0
    }
}

/// Gets the variant name string from the variant code
fn get_variant_name(variant_code: u8) -> &'static str {
    match variant_code {
        1 => "x86_64",
        2 => "x86_64_v2",
        3 => "x86_64_v3",
        4 => "x86_64_v4",
        _ => "unknown",
    }
}

/// Gets an extended description for an architecture variant
fn get_architecture_description(variant_code: u8) -> String {
    match variant_code {
        4 => "Experimental - optimized for AVX-512",
        3 => "Modern CPUs - optimized for AVX2",
        2 => "Recommended - optimized for SSE3",
        1 => "Universal - all x86-64 CPUs",
        _ => "Unknown",
    }
    .to_string()
}

/// Menu for selecting proton cachyos arch, returns selected or _v2 if in quick mode
pub fn select_architecture_variant(
    release_name: &str,
    variants: Vec<Download>,
    quick_mode: bool,
) -> Result<Download> {
    if variants.is_empty() {
        return Err(anyhow!("No architecture variants available"));
    }

    if quick_mode {
        let default = variants
            .iter()
            .find(|d| d.file_name.contains("_v2"))
            .or_else(|| variants.first())
            .unwrap();
        println!(
            "Selected {} by default",
            get_variant_name(get_architecture_variant(&default.file_name))
        );
        return Ok(default.clone());
    }

    // Create ArchitectureVariant objects with descriptions
    let arch_variants: Vec<ArchitectureVariant> = variants
        .iter()
        .filter_map(|download| {
            let variant_code = get_architecture_variant(&download.file_name);
            if variant_code == 0 {
                return None;
            }
            let variant_name = get_variant_name(variant_code).to_string();
            let description = get_architecture_description(variant_code);
            Some(ArchitectureVariant {
                name: variant_name,
                description,
                download: download.clone(),
            })
        })
        .collect();

    if arch_variants.is_empty() {
        return Ok(variants.into_iter().next().unwrap()); // fallback to first if none found
    }

    // Sort variants
    let mut sorted_variants = arch_variants;
    sorted_variants.sort_by(|a, b| {
        let order = |name: &str| -> u8 {
            match name {
                "x86_64" => 1,
                "x86_64_v2" => 2,
                "x86_64_v3" => 3,
                "x86_64_v4" => 4,
                _ => 99,
            }
        };
        order(&a.name).cmp(&order(&b.name))
    });

    let selected = Select::new(
        format!("Select CPU architecture for release '{}':", release_name).as_str(),
        sorted_variants,
    )
    .prompt()
    .unwrap_or_else(|_| std::process::exit(0));

    Ok(selected.download)
}
