//! CPU architecture detection and classification.
//!
//! This module detects the architecture of the running system and recognizes
//! arch names embedded in download file names (e.g. `x86_64`, `amd64`,`arm64`, `aarch64`).

use std::fmt;
use std::str::FromStr;

/// CPU architecture family used to select the correct download asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CpuArch {
    /// x86 family: `x86_64`, `x86`, `amd64`
    #[default]
    X86,
    /// Arm family: `aarch64`, `arm64`, `arm`
    Arm,
}

impl fmt::Display for CpuArch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CpuArch::X86 => write!(f, "amd64"),
            CpuArch::Arm => write!(f, "arm64"),
        }
    }
}

impl FromStr for CpuArch {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let lower = input.to_ascii_lowercase();
        if is_x86_token(&lower) {
            Ok(CpuArch::X86)
        } else if is_arm_token(&lower) {
            Ok(CpuArch::Arm)
        } else {
            Err(())
        }
    }
}

const X86_TOKENS: [&str; 4] = ["x86_64", "x86-64", "amd64", "x86"];
const ARM_TOKENS: [&str; 3] = ["aarch64", "arm64", "arm"];

fn is_x86_token(token: &str) -> bool {
    X86_TOKENS.contains(&token)
}

fn is_arm_token(token: &str) -> bool {
    ARM_TOKENS.contains(&token)
}

/// Detects the architecture of the running system using the target the binary
/// was compiled for.
pub fn detect_system_arch() -> CpuArch {
    CpuArch::from_str(std::env::consts::ARCH).unwrap_or_default()
}

/// Checks whether a character is a valid token delimiter for an architecture
/// name embedded in a file name.
fn is_delimiter(c: char) -> bool {
    c == '-' || c == '_' || c == '.'
}

/// Finds the architecture token present in a file name, if any.
///
/// Longest tokens are matched first so that e.g. `arm64` is not consumed by
/// `arm`, and `x86_64` is not consumed by `x86`. Tokens must be delimited by
/// `-`, `_`, `.`, or the start/end of the string.
fn find_arch_token(name: &str) -> Option<(usize, usize, CpuArch)> {
    let lower = name.to_ascii_lowercase();

    let mut all_tokens: Vec<(&str, CpuArch)> = X86_TOKENS
        .iter()
        .map(|t| (*t, CpuArch::X86))
        .chain(ARM_TOKENS.iter().map(|t| (*t, CpuArch::Arm)))
        .collect();
    all_tokens.sort_by_key(|(t, _)| std::cmp::Reverse(t.len()));

    for (token, arch) in all_tokens {
        let mut start = 0;
        while let Some(rel_pos) = lower[start..].find(token) {
            let abs_start = start + rel_pos;
            let abs_end = abs_start + token.len();

            let prev_ok = abs_start == 0 || {
                let c = lower[..abs_start].chars().next_back().unwrap();
                is_delimiter(c)
            };
            let next_ok = abs_end == lower.len() || {
                let c = lower[abs_end..].chars().next().unwrap();
                is_delimiter(c)
            };

            if prev_ok && next_ok {
                return Some((abs_start, abs_end, arch));
            }

            start = abs_start + 1;
        }
    }

    None
}

/// Extracts the architecture token from a file name, returning the arch and a
/// copy of the name with the token (and a single leading separator) removed.
pub fn arch_from_file_name(name: &str) -> Option<CpuArch> {
    find_arch_token(name).map(|(_, _, arch)| arch)
}

/// Returns the length of a trailing microarchitecture level
/// (e.g. `_v2`) at the given byte position, or `0` if none.
fn microarch_level_len(s: &str, from: usize) -> usize {
    let rest = &s[from..];
    if !rest.starts_with("_v") {
        return 0;
    }
    let digits = rest[2..].chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return 0;
    }
    2 + digits
}

/// Strips the architecture token from a file name.
///
/// The token may optionally be followed by a microarchitecture level (e.g.
/// `x86_64_v3`), which is stripped as well.
///
/// Returns the stripped name and the detected architecture (if any). If no
/// architecture token is found, the name is returned unchanged with `None`.
pub fn strip_arch_suffix(name: &str) -> (String, Option<CpuArch>) {
    let Some((start, end, arch)) = find_arch_token(name) else {
        return (name.to_owned(), None);
    };

    let end = end + microarch_level_len(name, end);

    let mut stripped = name.to_owned();
    // Remove the token and one preceding separator if present.
    let mut remove_start = start;
    if start > 0 {
        let prev = name[..start].chars().next_back().unwrap();
        if is_delimiter(prev) {
            remove_start = start - prev.len_utf8();
        }
    }
    stripped.replace_range(remove_start..end, "");

    (stripped, Some(arch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_from_file_name_x86() {
        assert_eq!(
            arch_from_file_name("GE-Proton11-4-x86_64.tar.gz"),
            Some(CpuArch::X86)
        );
        assert_eq!(arch_from_file_name("app-x86.tar.gz"), Some(CpuArch::X86));
        assert_eq!(arch_from_file_name("app-amd64.tar.gz"), Some(CpuArch::X86));
        assert_eq!(arch_from_file_name("app-x86-64.tar.gz"), Some(CpuArch::X86));
        assert_eq!(
            arch_from_file_name("proton-cachyos-x86_64_v3.tar.gz"),
            Some(CpuArch::X86)
        );
    }

    #[test]
    fn test_arch_from_file_name_arm() {
        assert_eq!(
            arch_from_file_name("GE-Proton11-4-aarch64.tar.gz"),
            Some(CpuArch::Arm)
        );
        assert_eq!(arch_from_file_name("app-arm64.tar.gz"), Some(CpuArch::Arm));
        assert_eq!(arch_from_file_name("app-arm.tar.gz"), Some(CpuArch::Arm));
        assert_eq!(
            arch_from_file_name("app-AArch64.tar.gz"),
            Some(CpuArch::Arm)
        );
    }

    #[test]
    fn test_arch_from_file_name_none() {
        assert_eq!(arch_from_file_name("GE-Proton11-3.tar.gz"), None);
        assert_eq!(arch_from_file_name("GE-Proton11-2.tar.gz"), None);
        assert_eq!(arch_from_file_name("dxvk-2.6.1.tar.gz"), None);
    }

    #[test]
    fn test_strip_arch_suffix() {
        let (stripped, arch) = strip_arch_suffix("GE-Proton11-4-x86_64.tar.gz");
        assert_eq!(stripped, "GE-Proton11-4.tar.gz");
        assert_eq!(arch, Some(CpuArch::X86));

        let (stripped, arch) = strip_arch_suffix("GE-Proton11-4-aarch64.tar.gz");
        assert_eq!(stripped, "GE-Proton11-4.tar.gz");
        assert_eq!(arch, Some(CpuArch::Arm));

        let (stripped, arch) = strip_arch_suffix("GE-Proton11-3.tar.gz");
        assert_eq!(stripped, "GE-Proton11-3.tar.gz");
        assert_eq!(arch, None);
    }

    #[test]
    fn test_strip_arch_suffix_with_microarch_level() {
        let (stripped, arch) = strip_arch_suffix("GE-Proton26-9-x86_64_v7.tar.gz");
        assert_eq!(stripped, "GE-Proton26-9.tar.gz");
        assert_eq!(arch, Some(CpuArch::X86));

        let (stripped, arch) = strip_arch_suffix("GE-Proton26-9-aarch64_v8.tar.gz");
        assert_eq!(stripped, "GE-Proton26-9.tar.gz");
        assert_eq!(arch, Some(CpuArch::Arm));

        let (stripped, arch) =
            strip_arch_suffix("proton-cachyos-9.0-20250101-abc123-x86_64_v3.tar.gz");
        assert_eq!(stripped, "proton-cachyos-9.0-20250101-abc123.tar.gz");
        assert_eq!(arch, Some(CpuArch::X86));
    }

    #[test]
    fn test_strip_arch_suffix_preserves_non_arch_substrings() {
        // "arm" inside a word should not be treated as an arch token.
        let (stripped, arch) = strip_arch_suffix("warm-wine-9.0.tar.xz");
        assert_eq!(stripped, "warm-wine-9.0.tar.xz");
        assert_eq!(arch, None);
    }

    #[test]
    fn test_detect_system_arch_is_valid() {
        let arch = detect_system_arch();
        assert!(matches!(arch, CpuArch::X86 | CpuArch::Arm));
    }

    #[test]
    fn test_from_str() {
        assert_eq!("amd64".parse::<CpuArch>(), Ok(CpuArch::X86));
        assert_eq!("x86_64".parse::<CpuArch>(), Ok(CpuArch::X86));
        assert_eq!("x86".parse::<CpuArch>(), Ok(CpuArch::X86));
        assert_eq!("arm64".parse::<CpuArch>(), Ok(CpuArch::Arm));
        assert_eq!("arm".parse::<CpuArch>(), Ok(CpuArch::Arm));
        assert_eq!("aarch64".parse::<CpuArch>(), Ok(CpuArch::Arm));
        assert_eq!("mips".parse::<CpuArch>(), Err(()));
        // maybe someday ?
        assert_eq!("riscv".parse::<CpuArch>(), Err(()));
    }

    #[test]
    fn test_display() {
        assert_eq!(CpuArch::X86.to_string(), "amd64");
        assert_eq!(CpuArch::Arm.to_string(), "arm64");
    }
}
