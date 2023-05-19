use crate::app::Message;
use iced::{widget::text, Element};
use std::{fmt, path::PathBuf};

/// Returns a list of
pub fn find_launchers() -> Vec<Launcher> {
    LauncherCollection::find_launchers()
        .into_iter()
        .filter(|l| l.has_data())
        .fold(vec![], |mut list, l| {
            list.push(l);
            list
        })
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Launcher {
    Lutris(Option<LauncherData>),
    LutrisFlatpak(Option<LauncherData>),
    Steam(Option<LauncherData>),
    SteamFlatpak(Option<LauncherData>),
}

impl fmt::Display for Launcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Launcher::Lutris(_data) => {
                    "Lutris:"
                }
                Launcher::LutrisFlatpak(_data) => {
                    "Lutris Flatpak:"
                }
                Launcher::Steam(_data) => {
                    "Steam:"
                }
                Launcher::SteamFlatpak(_data) => {
                    "Steam Flatpak:"
                }
            }
        )
    }
}

impl Launcher {
    /// Returns a bool, true if the Launcher has Some LauncheData, false otherwise
    fn has_data(&self) -> bool {
        match self {
            Launcher::Steam(data) => data.is_some(),
            Launcher::SteamFlatpak(data) => data.is_some(),
            Launcher::Lutris(data) => data.is_some(),
            Launcher::LutrisFlatpak(data) => data.is_some(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LauncherData {
    // Location of launcher's runner/wine folder
    pub path: PathBuf,
    pub installs: Vec<Install>,
}

impl LauncherData {
    /// Gets the Proton/Wine data from a Launcher's data, will return an empty list if the data is None
    pub fn get_installs_text_list(data: &Option<Self>) -> Vec<Element<Message>> {
        if let Some(data) = data {
            data.installs
                .iter()
                .map(|d| text(d.name.clone()).into())
                .collect()
        } else {
            vec![]
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Install {
    pub name: String,
}

pub struct LauncherCollection {
    curr: u32,
    steam: Launcher,
    steam_flatpak: Launcher,
    lutris: Launcher,
    lutris_flatpak: Launcher,
}

impl LauncherCollection {
    pub fn new() -> LauncherCollection {
        LauncherCollection {
            curr: 0,
            steam: Launcher::Steam(None),
            steam_flatpak: Launcher::SteamFlatpak(None),
            lutris: Launcher::Lutris(None),
            lutris_flatpak: Launcher::LutrisFlatpak(None),
        }
    }

    /// Find any launcher's wine or proton folder, return all found in a list
    pub fn find_launchers() -> LauncherCollection {
        // Get a list of the known wine proton folder locations
        let mut collection: LauncherCollection = LauncherCollection::new();

        get_launcher_data(&mut collection.steam);
        get_launcher_data(&mut collection.steam_flatpak);
        get_launcher_data(&mut collection.lutris);
        get_launcher_data(&mut collection.lutris_flatpak);

        collection
    }
}

impl Iterator for LauncherCollection {
    type Item = Launcher;

    fn next(&mut self) -> Option<Self::Item> {
        self.curr += 1;

        match self.curr - 1 {
            0 => Some(self.steam.clone()),
            1 => Some(self.steam_flatpak.clone()),
            2 => Some(self.lutris.clone()),
            3 => Some(self.lutris_flatpak.clone()),
            _ => None,
        }
    }
}

/// Modifies the passed in Launcher's data
fn get_launcher_data(launcher: &mut Launcher) {
    match launcher {
        Launcher::Steam(data) => {
            let pathbuf = libprotonup::utils::expand_tilde(PathBuf::from(
                libprotonup::constants::DEFAULT_STEAM_INSTALL_DIR,
            ))
            .unwrap();
            modify_launcher_data(pathbuf, data);
        }
        Launcher::SteamFlatpak(data) => {
            let pathbuf = libprotonup::utils::expand_tilde(PathBuf::from(
                libprotonup::constants::DEFAULT_STEAM_INSTALL_DIR_FLATPAK,
            ))
            .unwrap();
            modify_launcher_data(pathbuf, data);
        }
        Launcher::Lutris(data) => {
            let pathbuf = libprotonup::utils::expand_tilde(PathBuf::from(
                libprotonup::constants::DEFAULT_LUTRIS_INSTALL_DIR,
            ))
            .unwrap();
            modify_launcher_data(pathbuf, data);
        }
        Launcher::LutrisFlatpak(data) => {
            let pathbuf = libprotonup::utils::expand_tilde(PathBuf::from(
                libprotonup::constants::DEFAULT_LUTRIS_INSTALL_DIR_FLATPAK,
            ))
            .unwrap();
            modify_launcher_data(pathbuf, data);
        }
    }
}

/// Modifies the LauncherData part of any Launcher, each launcher's different path is passed in
fn modify_launcher_data(pathbuf: PathBuf, data: &mut Option<LauncherData>) {
    let dir_iter = pathbuf.read_dir();
    // If the read_dir returned an error, we'll set the LauncherData to None, otherwise continue
    if dir_iter.is_err() {
        eprintln!("Couldn't read passed in dir: {:?}\n{:?}", pathbuf, dir_iter);
        *data = None
    } else {
        // Create a list to hold all the Strings of different Proton/Wine versions
        let mut entry_list: Vec<Install> = vec![];
        for entry in dir_iter.unwrap() {
            // If there's an error reading any Proton/Wine release, skip it in the loop
            if let Ok(entry) = entry {
                if let Ok(name) = entry.file_name().into_string() {
                    entry_list.push(Install { name })
                }
            }
        }
        *data = Some(LauncherData {
            path: pathbuf,
            installs: entry_list,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modify_launcher_data() {
        let mut launcher_data: Option<LauncherData> = None;
        let pathbuf = libprotonup::utils::expand_tilde(PathBuf::from(
            libprotonup::constants::DEFAULT_STEAM_INSTALL_DIR,
        ))
        .unwrap();

        modify_launcher_data(pathbuf, &mut launcher_data);

        if let Some(launcher_data) = launcher_data {
            println!("Found installs: {:?}", launcher_data.installs)
        }
    }

    #[test]
    fn test_find_launcher() {
        let list = find_launchers();

        println!("Got list: {:?}", list);
    }
}
