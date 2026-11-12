#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::utils::match_version;

fuzz_target!(|data: &[u8]| {
    // Convert fuzz input to strings
    if let Ok(input) = std::str::from_utf8(data) {
        // Split input into two parts: version string and tag name
        // Use a delimiter that's unlikely in normal version strings
        if let Some(delimiter_pos) = input.find("|||") {
            let version_str = &input[..delimiter_pos];
            let tag_name = &input[delimiter_pos + 3..];

            // Run match_version - we're mainly testing for panics
            let _result = match_version(version_str, tag_name);

            // An empty version never matches anything.
            assert!(!match_version("", tag_name));

            // A release tag always matches itself.
            if !tag_name.is_empty() {
                assert!(match_version(tag_name, tag_name));
            }
        }
    }
});
