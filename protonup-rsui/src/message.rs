use std::path::PathBuf;

use crate::download::DownloadPhase;
use crate::download_task::{DownloadUpdate, ToolProgress};
use libprotonup::apps::AppInstallations;
use libprotonup::downloads::Release;
use libprotonup::sources::CompatTool;

/// Messages that drive the GUI state machine
#[derive(Debug, Clone)]
pub(crate) enum Message {
    ScanApps,
    AppsScanned(Vec<AppInstallations>),

    SelectQuickUpdate,
    SelectDownloadForSteam,
    SelectDownloadForLutris,

    AppInstallationDetected(AppInstallations),
    ToolSelected(usize),
    ToolSelectionConfirmed,

    VersionsFetched(Vec<libprotonup::downloads::Release>),
    ToggleVersion(usize),
    StartSelectedDownloads,

    SelectArchitecture(u8),

    AlreadyInstalledChecked(Vec<ToolDownload>),
    ToggleReinstall(usize),
    ConfirmReinstallSelection,

    DownloadUpdate(DownloadUpdate),

    QuickUpdateChecked(Vec<(String, bool)>),
    ForceReinstall,

    BackToInitial,
    BackToToolSelection,

    ToggleChangelog(Option<(Release, CompatTool)>),

    SelectionError(String),

    TickSpinner,

    Cancel,

    CloseRequested,

    SelectDownloadForCustom,
    CustomPathInput(String),
    OpenFolderPicker,
    FolderPicked(Option<PathBuf>),

    SelectManageInstallations,
    VersionToggled(usize, usize),
    DeleteSelectedVersions,
    DeleteCompleted(Result<Vec<String>, String>),
    VersionsScanned(Vec<(AppInstallations, Vec<(PathBuf, String)>)>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum GuiMode {
    #[default]
    Initial,
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
    DownloadForCustom,
    ManageInstallations,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum AppMode {
    #[default]
    None,
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
    DownloadForCustom,
    ManageInstallations,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum SelectionStep {
    #[default]
    Initial,
    SelectingTools,
    SelectingVersions,
    SelectingArchitecture,
    ConfirmReinstall,
    Downloading,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolDownload {
    pub name: String,
    pub phase: DownloadPhase,
    pub progress: f32,
    pub status: ToolStatus,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ToolStatus {
    #[default]
    Pending,
    Downloading,
    Validating,
    Unpacking,
    _Complete,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum QuickUpdateStatus {
    #[default]
    Idle,
    Checking,
    AllUpToDate(Vec<String>),
    InProgress,
    Complete,
}

impl ToolDownload {
    pub fn new(name: String) -> Self {
        Self {
            name,
            phase: DownloadPhase::DetectingApps,
            progress: 0.0,
            status: ToolStatus::Pending,
        }
    }

    pub fn update_from_progress(&mut self, progress: &ToolProgress) {
        self.phase = progress.phase.clone();
        self.progress = progress.percent;

        match &self.phase {
            DownloadPhase::DetectingApps | DownloadPhase::FetchingReleases => {
                self.status = ToolStatus::Pending;
            }
            DownloadPhase::Downloading => {
                self.status = ToolStatus::Downloading;
            }
            DownloadPhase::Validating => {
                self.status = ToolStatus::Validating;
            }
            DownloadPhase::Unpacking => {
                self.status = ToolStatus::Unpacking;
            }
            DownloadPhase::Complete => {
                self.status = ToolStatus::_Complete;
            }
            DownloadPhase::Error => {
                self.status = ToolStatus::Error(progress.status_message.clone());
            }
        }
    }

    pub fn status_text(&self) -> String {
        match &self.status {
            ToolStatus::Pending => format!("{} - Waiting...", self.name),
            ToolStatus::Downloading => {
                format!("{} - Downloading... {:.1}%", self.name, self.progress)
            }
            ToolStatus::Validating => {
                format!("{} - Validating... {:.1}%", self.name, self.progress)
            }
            ToolStatus::Unpacking => format!("{} - Installing... {:.1}%", self.name, self.progress),
            ToolStatus::_Complete => format!("{} - ✓ Installed", self.name),
            ToolStatus::Error(msg) => format!("{} - ✗ Error: {}", self.name, msg),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InstalledVersion {
    pub name: String,
    pub path: PathBuf,
    pub selected_for_deletion: bool,
}

#[derive(Debug)]
pub(crate) struct AppInstallationView {
    pub app: AppInstallations,
    pub versions: Vec<InstalledVersion>,
    pub loading: bool,
}
