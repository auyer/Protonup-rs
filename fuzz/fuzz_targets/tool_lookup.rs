#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::sources::CompatTool;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);

    // `--tool` accepts arbitrary argv strings. Parsing must never panic, and a
    // successful lookup must produce a fully populated, stable tool.
    if let Ok(tool) = input.parse::<CompatTool>() {
        assert!(!tool.name.is_empty(), "parsed tool has an empty name");
        assert_eq!(tool.to_string(), tool.name, "Display must equal name");

        // Normalization is stable: feeding the canonical name back in resolves
        // to the same tool.
        let round_trip = tool
            .name
            .parse::<CompatTool>()
            .expect("canonical tool name must parse");
        assert_eq!(round_trip.name, tool.name, "round trip changed the tool");

        // `normalize_tool_name` is case- and underscore-insensitive. An ASCII
        // uppercase variant of an accepted input must resolve identically.
        let upper = input.to_ascii_uppercase();
        let upper_tool = upper
            .parse::<CompatTool>()
            .expect("uppercase variant of an accepted tool must parse");
        assert_eq!(
            upper_tool.name, tool.name,
            "uppercase variant resolved to a different tool"
        );
    }
});
