#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::apps::{App, AppInstallations};
use libprotonup::sources::{CompatTool, CompatTools};
use libprotonup::utils::expand_tilde;
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    // First field is the `--for` value, optional second field overrides the
    // tool (falls back to GEProton, like the CLI).
    let mut fields = data.splitn(2, |byte| *byte == 0);
    let target = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
    let resolved_tool = fields
        .next()
        .map(|tool| String::from_utf8_lossy(tool).into_owned())
        .and_then(|tool| tool.parse::<CompatTool>().ok())
        .unwrap_or_else(|| {
            "GEProton"
                .parse::<CompatTool>()
                .expect("GEProton must be a known tool")
        });

    // `App::from_str_or_path` has an exact, case-insensitive mapping for the
    // built-in apps and preserves everything else verbatim as a custom path.
    let app = App::from_str_or_path(&target);
    match &app {
        App::Steam => assert_eq!(
            target.to_lowercase(),
            "steam",
            "steam app must only be produced for case-insensitive 'steam'"
        ),
        App::Lutris => assert_eq!(
            target.to_lowercase(),
            "lutris",
            "lutris app must only be produced for case-insensitive 'lutris'"
        ),
        App::Custom(path) => {
            assert_eq!(path, &target, "custom path must be preserved verbatim");
        }
    }
    // The resolved tool must be one of the real bundled tools.
    assert!(
        CompatTools
            .iter()
            .any(|tool| tool.name == resolved_tool.name),
        "resolved tool is not part of the bundled tool list"
    );

    // Custom installation paths flow straight into `expand_tilde`, so the
    // default dir must be preserved and the installation dir must agree with
    // `expand_tilde` for every real tool.
    let installation = AppInstallations::new_custom_app_install(target.clone());
    assert_eq!(
        installation.as_app(),
        App::Custom(target.clone()),
        "custom installation must round trip to a custom app"
    );
    assert_eq!(
        installation.default_install_dir().as_str(),
        target.as_str(),
        "custom installation must report the input path verbatim"
    );

    let expected = expand_tilde(Path::new(target.as_str()));
    for tool in CompatTools.iter() {
        assert_eq!(
            installation.installation_dir(tool),
            expected,
            "installation_dir for custom path {:?} and tool {:?} disagrees with expand_tilde",
            target,
            tool.name
        );
    }
});
