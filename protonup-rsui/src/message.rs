use std::path::PathBuf;

use crate::download::DownloadPhase;
use crate::download_task::{DownloadUpdate, ToolProgress};
use libprotonup::apps::AppInstallations;
use libprotonup::architecture_variants::MicroArchVariants;
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
    SelectCheckWhatsNew,

    AppInstallationDetected(AppInstallations),
    /// Selects a tool and immediately advances (list-button click).
    ToolChosen(usize),
    ToolSelectionConfirmed,

    VersionsFetched(Vec<libprotonup::downloads::Release>),
    ToggleVersion(usize),
    VersionChosen(usize),
    StartSelectedDownloads,

    /// Selects an architecture and immediately starts downloading (list-button click).
    ArchitectureChosen(MicroArchVariants),

    AlreadyInstalledChecked(Vec<ToolDownload>),
    ToggleReinstall(usize),
    ConfirmReinstallSelection,

    DownloadUpdate(DownloadUpdate),

    QuickUpdateChecked(Vec<InstalledTool>),
    ForceReinstall,

    BackToInitial,
    BackToToolSelection,

    ToggleChangelog(Box<Option<(Release, CompatTool)>>),

    ToggleInstalledVersions(Box<Option<InstalledTool>>),

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

    /// A navigation action requested by the keyboard or a gamepad.
    Nav(NavAction),
    /// A right-stick scroll request: a continuous pixel delta applied to the
    /// scrollable that currently holds focus.
    Scroll {
        x: f32,
        y: f32,
    },
    /// The gamepad backend could not be initialized; controller support is disabled.
    GamepadUnavailable,
    /// Toggle the keyboard shortcuts help panel.
    ToggleKeyboardShortcuts,
    /// Toggle the controller shortcuts help panel.
    ToggleControllerShortcuts,
    /// Close any open shortcuts help panel.
    CloseShortcuts,
}

/// A focusable region of the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pane {
    /// The left navigation sidebar.
    Sidebar,
    /// The center content area.
    Main,
    /// The bottom action bar.
    Actions,
}

impl Pane {
    /// The next pane in the focus cycle (wraps around).
    pub fn next(self) -> Self {
        match self {
            Self::Sidebar => Self::Main,
            Self::Main => Self::Actions,
            Self::Actions => Self::Sidebar,
        }
    }

    /// The previous pane in the focus cycle (wraps around).
    pub fn previous(self) -> Self {
        match self {
            Self::Sidebar => Self::Actions,
            Self::Main => Self::Sidebar,
            Self::Actions => Self::Main,
        }
    }
}

/// A user request to navigate or act via keyboard/controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavAction {
    /// Move focus to the next pane (Tab / right shoulder / right trigger).
    FocusNextPane,
    /// Move focus to the previous pane (Shift+Tab / left shoulder / left trigger).
    FocusPreviousPane,
    /// Move focus to the previous option vertically (ArrowUp / D-pad up / stick up).
    MoveUp,
    /// Move focus to the next option vertically (ArrowDown / D-pad down / stick down).
    MoveDown,
    /// Move focus to the previous option horizontally, or the previous pane
    /// when there is no horizontal neighbor (ArrowLeft / D-pad left / stick left).
    MoveLeft,
    /// Move focus to the next option horizontally, or the next pane when there
    /// is no horizontal neighbor (ArrowRight / D-pad right / stick right).
    MoveRight,
    /// Activate the focused option (Space / A / Cross).
    Select,
    /// Trigger the current view's primary "advance" action (Enter / Start).
    Primary,
}

/// A focusable option within a pane, in rendering order.
#[derive(Debug, Clone)]
pub(crate) struct Focusable {
    /// The message to send when the option is activated.
    pub message: Message,
    /// For text inputs, the iced widget id to focus instead of sending a message.
    pub widget_focus: Option<iced::widget::Id>,
}

impl Focusable {
    pub(crate) fn new(message: Message) -> Self {
        Self {
            message,
            widget_focus: None,
        }
    }

    pub(crate) fn widget(widget_focus: impl Into<iced::widget::Id>) -> Self {
        Self {
            message: Message::TickSpinner, // never dispatched; widget focus is used instead
            widget_focus: Some(widget_focus.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum GuiMode {
    #[default]
    Initial,
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
    DownloadForCustom,
    CheckWhatsNew,
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
    CheckWhatsNew,
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
    AllUpToDate(Vec<InstalledTool>),
    InProgress,
    Complete,
}

/// A tool (and its installed versions) detected for one app during Quick Update.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InstalledTool {
    pub app: AppInstallations,
    pub tool: String,
    /// The latest installed version of the app's default tool, if any.
    pub latest: Option<String>,
    /// All installed versions for this app (default tool + other tools), newest first.
    pub versions: Vec<String>,
}

impl InstalledTool {
    /// Number of installed entries other than the shown "latest" one.
    pub fn other_count(&self) -> usize {
        self.versions.len().saturating_sub(1)
    }
}

/// Digit-aware (natural) ordering for version strings such as "GE-Proton11-11",
/// so that "GE-Proton11-2" < "GE-Proton11-11".
pub(crate) fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    fn chunks(s: &str) -> impl Iterator<Item = (bool, &str)> {
        let bytes = s.as_bytes();
        let mut i = 0usize;
        std::iter::from_fn(move || {
            if i >= bytes.len() {
                return None;
            }
            let is_digit = bytes[i].is_ascii_digit();
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() == is_digit {
                j += 1;
            }
            let chunk = &s[i..j];
            i = j;
            Some((is_digit, chunk))
        })
    }

    fn cmp_digit_runs(a: &str, b: &str) -> std::cmp::Ordering {
        let a = a.trim_start_matches('0');
        let b = b.trim_start_matches('0');
        a.len().cmp(&b.len()).then_with(|| a.cmp(b))
    }

    let mut ca = chunks(a);
    let mut cb = chunks(b);
    loop {
        match (ca.next(), cb.next()) {
            (Some((true, x)), Some((true, y))) => {
                let c = cmp_digit_runs(x, y);
                if c != std::cmp::Ordering::Equal {
                    return c;
                }
            }
            (Some((ta, x)), Some((tb, y))) => {
                let c = x.cmp(y).then_with(|| ta.cmp(&tb));
                if c != std::cmp::Ordering::Equal {
                    return c;
                }
            }
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
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
