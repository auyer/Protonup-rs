use iced::{Element, Subscription, Task};

pub(crate) mod message;
pub(crate) mod state;
pub(crate) mod update;
pub(crate) mod views;

mod circular;
pub(crate) mod download;
pub(crate) mod download_task;
mod easing;
pub(crate) mod input;

#[cfg(test)]
mod gui_tests;

// Re-exports needed by tests at crate root
#[cfg(test)]
pub(crate) use crate::download::DownloadPhase;
#[cfg(test)]
pub(crate) use crate::download_task::{DownloadUpdate, ToolProgress};
#[cfg(test)]
pub(crate) use crate::message::{
    AppMode, GuiMode, InstalledTool, Message, NavAction, Pane, QuickUpdateStatus, SelectionStep,
    ToolDownload, ToolStatus,
};
#[cfg(test)]
pub(crate) use crate::state::{FocusState, ProtonupGui};
#[cfg(test)]
pub(crate) use libprotonup::apps::AppInstallations;

pub(crate) const LOGO_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/protonup-rs-logo.png"
));

#[derive(Default)]
struct App(state::ProtonupGui);

impl App {
    fn update(&mut self, msg: message::Message) -> Task<message::Message> {
        update::handle(&mut self.0, msg)
    }

    fn view(&self) -> Element<'_, message::Message> {
        views::app_view(&self.0)
    }

    fn title(&self) -> String {
        self.0.window_title()
    }

    fn subscription(&self) -> Subscription<message::Message> {
        self.0.subscription()
    }
}

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .title(App::title)
        .window(iced::window::Settings {
            // Start at the Steam Deck resolution (1280x800).
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(512.0, 360.0)),
            maximized: false,
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "protonup-rsui".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
