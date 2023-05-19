use iced::executor;
use iced::widget::{button, column, container, pick_list, progress_bar, row, text, Column};
use iced::{
    Application,
    Command,
    Element,
    Length,
    Subscription,
    Theme,
    // Background,
    // Color,
};

use crate::utility::{self, Launcher, LauncherData};

//use std::{cmp, path::PathBuf};

#[derive(Debug)]
pub struct App {
    launchers: Vec<Launcher>,
    selected_launcher: Option<Launcher>,
}

#[derive(Debug, Clone)]
pub enum Message {
    QuickUpdate,
    LauncherSelected(Launcher),
}

impl Application for App {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                // launchers: vec![
                //     Launcher::Steam(Some(LauncherData {
                //         path: PathBuf::new(),
                //         installs: vec![],
                //     })),
                //     Launcher::Lutris(Some(LauncherData {
                //         path: PathBuf::new(),
                //         installs: vec![],
                //     })),
                // ],
                launchers: utility::find_launchers(),
                selected_launcher: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Protonup-rs")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // TODO
            Message::QuickUpdate => {}
            Message::LauncherSelected(launcher) => self.selected_launcher = Some(launcher),
        };

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
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
            column(
                // vec![text("List of Downloaded Proton/Wine versions").into(), text("Version 1.1").into(), text("Version 1.2").into(),]
                if let Some(launcher) = &self.selected_launcher {
                    match launcher {
                        Launcher::Lutris(data) => LauncherData::get_installs_text_list(data),
                        Launcher::LutrisFlatpak(data) => LauncherData::get_installs_text_list(data),
                        Launcher::Steam(data) => LauncherData::get_installs_text_list(data),
                        Launcher::SteamFlatpak(data) => LauncherData::get_installs_text_list(data),
                    }
                } else {
                    vec![]
                },
            )
            .width(Length::FillPortion(4))
            .padding(5),
        );

        let content = column(vec![
            container(
                pick_list(
                    self.launchers.clone(),
                    self.selected_launcher.clone(),
                    Message::LauncherSelected,
                )
                .width(Length::Fill),
            )
            .height(Length::Units(40))
            .width(Length::Fill)
            // Will figure out how to fix later
            // .style(container::Style {
            //     background: Some(iced::Background::Color(iced::Color {
            //         r: 10.0,
            //         g: 11.0,
            //         b: 32.0,
            //         a: 0.0,
            //     })),
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
