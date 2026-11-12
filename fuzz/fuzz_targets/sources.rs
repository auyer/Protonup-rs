#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::sources::{CompatTool, Forge, ToolType};

fuzz_target!(|data: &[u8]| {
    if let Ok(utf8) = str::from_utf8(data) {
        let utf8 = utf8.to_owned();

        let template = format!("{}-{{version}}", utf8.clone());

        let s = CompatTool::new_custom(
            utf8.clone(),
            Forge::GitHub,
            utf8.clone(),
            utf8.clone(),
            ToolType::Runtime,
            None,
            Some((utf8.clone(), utf8.clone())),
            Some(template.to_owned()),
        );

        assert!(format!("{s}") == utf8);
        assert!(s.installation_name("v1").contains("v1"));
        assert!(s.filter_asset("v1"));

        // Fuzz the version argument too: with a no-op replacement the
        // "{prefix}-{version}" template must always contain the version.
        let installation_name = s.installation_name(&utf8);
        assert!(
            installation_name.contains(&utf8),
            "installation_name({utf8:?}) = {installation_name:?} lost the version"
        );
    }
});
