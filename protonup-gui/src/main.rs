use iced::{window, Application, Settings};

mod app;
mod utility;
use app::App;

pub fn main() -> iced::Result {
    App::run(Settings {
        window: window::Settings {
            size: (800, 450),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
