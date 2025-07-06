use inquire::MultiSelect;
use libprotonup::{
    apps::{self},
    files::{self, Folders},
};
use std::fmt;

use super::helper_menus::{confirm_menu, multiple_select_menu};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum ManageAppsMenuOptions {
    DetectAll,
    AppInstallations(apps::AppInstallations),
}

/// APP_VARIANTS_WITH_DETECT contains all variants of the App enum including the DetectAll variant
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
            Self::AppInstallations(app_inst) => write!(f, "{app_inst}"),
        }
    }
}

/// Prompt the user for which App they want to manage
fn manage_menu() -> Vec<ManageAppsMenuOptions> {
    let answer = MultiSelect::new(
        "Select the Applications you want to manage :",
        APP_VARIANTS_WITH_DETECT.to_vec(),
    )
    .with_default(&[0_usize])
    .prompt();

    answer.unwrap_or_else(|_| {
        println!("The tag list could not be processed");
        vec![]
    })
}

/// Allow the user to delete existing wine versions
///
/// The user selects the apps and wine versions to remove
pub(crate) async fn manage_apps_routine() {
    let choices = manage_menu();

    // default to all apps
    let mut selected_apps = apps::APP_INSTALLATIONS_VARIANTS.to_vec();
    if !choices.contains(&ManageAppsMenuOptions::DetectAll) {
        selected_apps = choices
            .iter()
            .map(|choice| match choice {
                ManageAppsMenuOptions::DetectAll => unreachable!(), // managed by the default case
                ManageAppsMenuOptions::AppInstallations(app) => app.to_owned(),
            })
            .collect::<Vec<apps::AppInstallations>>();
    }
    for app in selected_apps {
        let versions = match app.list_installed_versions().await {
            Ok(versions) => versions,
            Err(_) => {
                println!("App {app} not found in your system, skipping... ");
                continue;
            }
        };
        if versions.is_empty() {
            println!("No versions found for {app}, skipping... ");
            continue;
        }
        let delete_versions = multiple_select_menu(
            &format!("Select the versions you want to DELETE from {app}"),
            versions,
        )
        .unwrap_or_else(|_| vec![]);

        if delete_versions.is_empty() {
            println!("Zero versions selected for {app}, skipping...\n");
            continue;
        }
        let delete_versions = Folders(delete_versions);
        if confirm_menu(
            format!("Are you sure you want to delete {delete_versions} ?"),
            format!("If you choose yes, you will them from {app}"),
            true,
        ) {
            for version in delete_versions.0 {
                let version = version.0;
                let version_path = version.0.join(&version.1);
                files::remove_dir_all(&version_path).await.map_or_else(
                    |e| eprintln!("Error deleting {}: {}", version_path.as_path().display(), e),
                    |_| {
                        println!(
                            "{} {} deleted successfully",
                            &app,
                            version_path.as_path().display()
                        );
                    },
                );
            }
        }
    }
}
