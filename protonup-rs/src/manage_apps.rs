use inquire::MultiSelect;
use libprotonup::{apps, files};
use std::fmt;

use super::helper_menus::{confirm_menu, tag_menu};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum ManageAppsMenuOptions {
    DetectAll,
    AppInstallations(apps::AppInstallations),
}

// APP_VARIANTS_WITH_DETECT contains all variants of the App enum including the DetectAll variant
static APP_VARIANTS_WITH_DETECT: &[ManageAppsMenuOptions] = &[
    ManageAppsMenuOptions::DetectAll,
    ManageAppsMenuOptions::AppInstallations(apps::AppInstallations::Steam),
    ManageAppsMenuOptions::AppInstallations(apps::AppInstallations::SteamFlatpak),
    ManageAppsMenuOptions::AppInstallations(apps::AppInstallations::Lutris),
    ManageAppsMenuOptions::AppInstallations(apps::AppInstallations::LutrisFlatpak),
];

impl fmt::Display for ManageAppsMenuOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::DetectAll => write!(f, "Detect All"),
            Self::AppInstallations(app_inst) => write!(f, "{}", app_inst),
        }
    }
}

fn manage_menu() -> Vec<ManageAppsMenuOptions> {
    let answer = MultiSelect::new(
        "Select the Applications you want to manage :",
        APP_VARIANTS_WITH_DETECT.to_vec(),
    )
    .with_default(&[0_usize])
    .prompt();

    match answer {
        Ok(list) => list,

        Err(_) => {
            println!("The tag list could not be processed");
            vec![]
        }
    }
}

pub(crate) fn manage_apps_routine() {
    let mut apps = vec![];

    let choices = manage_menu();

    if choices.contains(&ManageAppsMenuOptions::DetectAll) {
        apps = apps::APP_INSTALLATIONS_VARIANTS.to_vec();
    }
    for app in apps {
        let versions = match app.list_installed_versions() {
            Ok(versions) => versions,
            Err(_) => {
                println!("App {} not found in your system, skipping... ", app);
                continue;
            }
        };
        if versions.is_empty() {
            println!("No versions found for {}, skipping... ", app);
            continue;
        }
        let delete_versions = match tag_menu(
            &format!("Select the versions you want to DELETE from {}", app),
            versions,
        ) {
            Ok(versions) => versions,
            Err(_) => {
                vec![]
            }
        };

        if delete_versions.is_empty() {
            println!("Zero versions selected for {}, skipping...\n", app);
            continue;
        }
        if confirm_menu(
            format!("Are you sure you want to delete {:?} ?", delete_versions),
            format!("If you choose yes, you will them from {}", app),
            true,
        ) {
            for version in delete_versions {
                files::remove_dir_all(&format!("{}{}", &app.default_install_dir(), &version))
                    .map_or_else(
                        |e| {
                            eprintln!(
                                "Error deleting {}{}: {}",
                                &app.default_install_dir(),
                                &version,
                                e
                            )
                        },
                        |_| {
                            println!("{} {} deleted successfully", &app, &version);
                        },
                    );
            }
        }
    }
}
