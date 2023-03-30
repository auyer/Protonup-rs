use iced::executor;
use iced::widget::{button, column, container, progress_bar, row, text, Column, pick_list};
use iced::{
    window, Alignment, Application, Command, Element, Length, Settings, Subscription, Theme, Color, Background
};
use std::{fmt, cmp};

mod download;
mod utility;

pub fn main() -> iced::Result {
    App::run(Settings {
        window: window::Settings {
            size: (800, 450),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}

#[derive(Debug)]
struct App {
    libraries: Vec<Library>, 
    selected_library: Option<Library>, 
    downloads: Vec<Download>,
    last_id: usize,
}

#[derive(Debug, Clone)]
pub enum Message {
    Add,
    Download(usize),
    DownloadProgressed((usize, download::Progress)),
    QuickUpdate,
    LibrarySelected(Library),
}

impl Application for App {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                libraries: vec![Library::Steam(LibraryData{}), Library::Lutris(LibraryData{})],
                downloads: vec![Download::new(0)],
                last_id: 0,
                selected_library: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Download progress - Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Add => {
                self.last_id += 1;

                self.downloads.push(Download::new(self.last_id));
            }
            Message::Download(index) => {
                if let Some(download) = self.downloads.get_mut(index) {
                    download.start();
                }
            }
            Message::DownloadProgressed((id, progress)) => {
                if let Some(download) = self.downloads.iter_mut().find(|download| download.id == id)
                {
                    download.progress(progress);
                }
            }
            // TODO
            Message::QuickUpdate => {},
            Message::LibrarySelected(library) => {self.selected_library = Some(library)},

        };

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(self.downloads.iter().map(Download::subscription))
    }

    fn view(&self) -> Element<Message> {
        let controls = Element::from(
            column(vec![button("TODO: Quick Update")
                .on_press(Message::QuickUpdate)
                .into()])
            .width(Length::FillPortion(1))
            .padding(5),
        );

        // TODO: will have a function to check the currently selected launcher based on the dropdown for already installed versions adding them to the list to be viewed
        let list = Element::from(
            column(vec![
                text("List of Downloaded Proton/Wine versions").into(),
                text("Version 1.1").into(),
                text("Version 1.2").into(),
            ])
            .width(Length::FillPortion(4))
            .padding(5),
        );

        let content = column(vec![
            container(pick_list(self.libraries.clone(), self.selected_library.clone(), Message::LibrarySelected))
                .height(Length::Units(40))
                // Will figure out how to fix later
                // .style( container::Style {  
                //     background: Some(iced::Background::Color( iced::Color {r: 10.0, g: 11.0, b: 32.0, a: 0.0})),
                //     ..Default::default()
                // })
                .into(),
            container(row(vec![controls, list]))
                .height(Length::Fill)
                .into(),
        ]);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Library {
    Lutris(LibraryData),
    LutrisFlatpak(LibraryData),
    Steam(LibraryData),
    SteamFlatpak(LibraryData),
}

impl fmt::Display for Library {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Library::Lutris(_data) => {"Lutris:"},
            Library::LutrisFlatpak(_data) => {"Lutris Flatpak:"},
            Library::Steam(_data) => {"Steam:"},
            Library::SteamFlatpak(_data) => {"Steam Flatpak:"},
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LibraryData {

}


#[derive(Debug)]
struct Download {
    id: usize,
    state: State,
}

#[derive(Debug)]
enum State {
    Idle,
    Downloading { progress: f32 },
    Finished,
    Errored,
}

impl Download {
    pub fn new(id: usize) -> Self {
        Download {
            id,
            state: State::Idle,
        }
    }

    pub fn start(&mut self) {
        match self.state {
            State::Idle { .. } | State::Finished { .. } | State::Errored { .. } => {
                self.state = State::Downloading { progress: 0.0 };
            }
            _ => {}
        }
    }

    pub fn progress(&mut self, new_progress: download::Progress) {
        if let State::Downloading { progress } = &mut self.state {
            match new_progress {
                download::Progress::Started => {
                    *progress = 0.0;
                }
                download::Progress::Advanced(percentage) => {
                    *progress = percentage;
                }
                download::Progress::Finished => {
                    self.state = State::Finished;
                }
                download::Progress::Errored => {
                    self.state = State::Errored;
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.state {
            State::Downloading { .. } => {
                download::file(self.id, "https://speed.hetzner.de/100MB.bin?")
                    .map(Message::DownloadProgressed)
            }
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let current_progress = match &self.state {
            State::Idle { .. } => 0.0,
            State::Downloading { progress } => *progress,
            State::Finished { .. } => 100.0,
            State::Errored { .. } => 0.0,
        };

        let progress_bar = progress_bar(0.0..=100.0, current_progress);

        let control: Element<_> = match &self.state {
            State::Idle => button("Start the download!")
                .on_press(Message::Download(self.id))
                .into(),
            State::Finished => {
                iced::widget::column![text("Download finished!"), button("Start again")]
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .into()
            }
            State::Downloading { .. } => {
                text(format!("Downloading... {:.2}%", current_progress)).into()
            }
            State::Errored => iced::widget::column![
                "Something went wrong :(",
                button("Try again").on_press(Message::Download(self.id)),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
            .into(),
        };

        Column::new()
            .spacing(10)
            .padding(10)
            .align_items(Alignment::Center)
            .push(progress_bar)
            .push(control)
            .into()
    }
}
