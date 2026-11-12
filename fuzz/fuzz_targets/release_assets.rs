#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::apps::AppInstallations;
use libprotonup::architecture::CpuArch;
use libprotonup::downloads::{Download, Release, group_and_dedup_releases};
use libprotonup::sources::{CompatTool, CompatTools};

fuzz_target!(|data: &[u8]| {
    // Production (`list_releases`) deserializes a `Vec<Release>`. Accepting a
    // bare release as a fallback gives the fuzzer an easier hill to climb.
    let releases: Vec<Release> = serde_json::from_slice(data)
        .or_else(|_| serde_json::from_slice::<Release>(data).map(|release| vec![release]))
        .unwrap_or_default();

    let app = AppInstallations::new_custom_app_install("/tmp/x".into());
    let mut entries: Vec<(Download, Release, AppInstallations, CompatTool)> = Vec::new();

    for tool in CompatTools.iter() {
        for arch in [CpuArch::X86, CpuArch::Arm] {
            for release in &releases {
                let one = release.get_download_info(&app, tool, arch);

                // A selected download must not be half-populated: a file name
                // without a URL can never be fetched.
                if !one.file_name.is_empty() {
                    assert!(
                        !one.download_url.is_empty(),
                        "get_download_info selected {:?} with an empty download URL",
                        one.file_name
                    );
                }
                entries.push((one, release.clone(), app.clone(), tool.clone()));

                let many = release.get_all_download_variants(&app, tool, arch);
                for download in many {
                    assert!(
                        !download.file_name.is_empty(),
                        "get_all_download_variants produced a download with an empty file name"
                    );
                    entries.push((download, release.clone(), app.clone(), tool.clone()));
                }
            }
        }
    }

    let entry_count = entries.len();
    let (groups, unique_releases) = group_and_dedup_releases(entries);

    for (key, (download, _, _, _)) in &groups {
        assert_eq!(
            download.dedup_key(),
            *key,
            "group map key does not match the grouped download's dedup key"
        );
    }
    assert!(
        unique_releases.len() <= entry_count,
        "deduplication produced more unique releases than input entries"
    );
});
