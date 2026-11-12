#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::architecture::strip_arch_suffix;
use libprotonup::sources::{AssetFilter, CompatTools};

fuzz_target!(|data: &[u8]| {
    // Layout: [selector byte][first string][0xff][second string]
    let (selector, rest) = match data.split_first() {
        Some((selector, rest)) => (*selector % 4, rest),
        None => (0, &[][..]),
    };
    let mut strings = rest.splitn(2, |byte| *byte == 0xff);
    let first = String::from_utf8_lossy(strings.next().unwrap_or_default()).into_owned();
    let second = String::from_utf8_lossy(strings.next().unwrap_or_default()).into_owned();

    let filter = match selector {
        0 => Some(AssetFilter::Prefix(first.clone())),
        1 => Some(AssetFilter::Prefixes(vec![first.clone(), second.clone()])),
        2 => Some(AssetFilter::PrefixSuffix(first.clone(), second.clone())),
        _ => None,
    };

    // Self-match invariants for every filter shape.
    if let Some(filter) = &filter {
        match filter {
            AssetFilter::Prefix(prefix) => {
                assert!(
                    filter.matches(prefix),
                    "Prefix must match its own prefix {prefix:?}"
                );
                let extended = format!("{prefix}{second}");
                assert!(
                    filter.matches(&extended),
                    "Prefix must match its prefix followed by arbitrary text"
                );
            }
            AssetFilter::Prefixes(prefixes) => {
                for prefix in prefixes {
                    assert!(
                        filter.matches(prefix),
                        "Prefixes must match each of its prefixes {prefix:?}"
                    );
                }
            }
            AssetFilter::PrefixSuffix(prefix, suffix) => {
                let combined = format!("{prefix}{suffix}");
                assert!(
                    filter.matches(&combined),
                    "PrefixSuffix must match its own prefix+suffix"
                );
            }
        }
    }

    // Production path: every bundled tool must filter arbitrary asset names
    // without panicking, and arch reporting must agree with strip_arch_suffix.
    let name = second.as_str();
    let (stripped, stripped_arch) = strip_arch_suffix(name);

    for tool in CompatTools.iter() {
        let (matched, arch) = tool.filter_asset_with_arch(name);
        assert_eq!(
            arch, stripped_arch,
            "filter_asset_with_arch reported a different arch than strip_arch_suffix"
        );
        assert_eq!(
            matched,
            tool.filter_asset(name) || (stripped != name && tool.filter_asset(stripped.as_str())),
            "filter_asset_with_arch does not compose filter_asset with arch stripping"
        );

        if tool.release_asset_filter.is_none() {
            assert!(
                tool.filter_asset(name),
                "a tool without an asset filter must accept every asset name"
            );
            assert!(
                matched,
                "a tool without an asset filter must accept every asset name (with arch)"
            );
        }
    }

    // The fuzz-built filter is a public API surface as well; applying it must
    // not panic even when no tool uses this exact shape.
    if let Some(filter) = &filter {
        let _ = filter.matches(name);
    }
});
