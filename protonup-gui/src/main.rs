use futures::FutureExt;
use gtk::prelude::*;
use libprotonup::{constants, file, github, utils};
use relm4::*;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::{sync::Arc, time::Duration};
fn main() {
    RelmApp::new("mc.rcpassos.ProtonUp-rs").run::<App>("Protonup-rs".into());
}

#[derive(Default)]
pub struct App {
    /// Tracks progress status
    download_size: u64,
    computing: bool,

    /// Contains output of a completed task.
    task: Option<CmdOut>,
}

pub struct Widgets {
    button: gtk::Button,
    button2: gtk::Button,
    label: gtk::Label,
    progress: gtk::ProgressBar,
}

#[derive(Debug)]
pub enum Input {
    Compute,
}

#[derive(Debug)]
pub enum Output {
    Clicked(u32),
}

#[derive(Debug)]
pub enum CmdOut {
    /// Progress update from a command.
    Progress(u64),
    /// The final output of the command.
    MessageUpdate(String),
    /// The final output of the command.
    Finished(String),
}

impl Component for App {
    type CommandOutput = CmdOut;
    type Init = String;
    type Input = Input;
    type Output = Output;
    type Root = gtk::Window;
    type Widgets = Widgets;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        _args: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        relm4::view! {
            container = gtk::Box {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_width_request: 300,
                set_spacing: 12,
                set_margin_top: 4,
                set_margin_bottom: 4,
                set_margin_start: 12,
                set_margin_end: 12,
                set_orientation: gtk::Orientation::Vertical,
                gtk::Box {
                    set_spacing: 4,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Horizontal,

                    append: button = &gtk::Button {
                        set_label: "Quick Update (Get latest GE Proton)",
                        connect_clicked[sender] => move |_| {
                            sender.input(Input::Compute);
                        }
                    },
                    append: button2 = &gtk::Button {
                        set_label: "Show Versions",
                        connect_clicked[sender] => move |_| {
                            sender.input(Input::Compute);
                        }
                    },
                },
                gtk::Box {
                    set_spacing: 4,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Vertical,

                    append: label = &gtk::Label {
                        set_xalign: 0.1,
                        set_label: "Chose an option",
                    },
                    append: progress = &gtk::ProgressBar {
                        set_visible: false,
                    }
                }
            }
        }

        root.set_child(Some(&container));

        ComponentParts {
            model: App::default(),
            widgets: Widgets {
                label,
                button,
                button2,
                progress,
            },
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            Input::Compute => {
                self.computing = true;

                let mut runtime =
                    tokio::runtime::Runtime::new().expect("Unable to create a runtime");
                let download = runtime
                    .block_on(github::fetch_data_from_tag("latest"))
                    .unwrap();
                // there must be a better way of doing this
                // What is happening here is: we are getting the download data from the lib to set the download size into this variable.
                // this is done because progress in this module is a float from 0 to 1, and the lib is reporting it in bytes.
                // I tested creating download_file as a method to App, but I couldnt use it in the async clojure (moved self, lifetime something...)
                self.download_size = download.size.clone();
                sender.command(|out, shutdown| {
                    shutdown
                        // Performs this operation until a shutdown is triggered
                        .register(async move {
                            let wait_time = Duration::from_millis(50); // 50ms wait is about 20Hz
                            let (progress, done) = file::create_progress_trackers();
                            let progress_read = Arc::clone(&progress);
                            let done_read = Arc::clone(&done);
                            let out_clone = out.clone();
                            thread::spawn(move || {
                                loop {
                                    let newpos = progress_read.load(Ordering::Relaxed);

                                    out_clone.send(CmdOut::Progress(newpos as u64));
                                    if done_read.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    thread::sleep(std::time::Duration::from_millis(10));

                                    thread::sleep(wait_time);
                                }
                                out_clone.send(CmdOut::MessageUpdate(
                                    "Checking file integrity".to_string(),
                                ));
                            });
                            download_file(
                                download,
                                constants::DEFAULT_INSTALL_DIR.to_string(),
                                progress,
                                done,
                            )
                            .await;

                            out.send(CmdOut::Finished("Downloaded".to_string()));
                            println!("Downloaded")
                        })
                        // Perform task until a shutdown interrupts it
                        .drop_on_shutdown()
                        // Wrap into a `Pin<Box<Future>>` for return
                        .boxed()
                });
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, _sender: ComponentSender<Self>) {
        if let CmdOut::Finished(_) = message {
            self.computing = false;
        }

        self.task = Some(message);
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.button.set_sensitive(!self.computing);

        if let Some(ref progress) = self.task {
            match progress {
                CmdOut::Progress(p) => {
                    widgets.label.set_label("Downloading...");
                    widgets.progress.show();
                    widgets
                        .progress
                        .set_fraction((*p as f64 / self.download_size as f64));
                }
                CmdOut::MessageUpdate(message) => {
                    widgets.label.set_label(message);
                }

                CmdOut::Finished(result) => {
                    widgets.progress.hide();
                    widgets.label.set_label(result);
                }
            }
        }
    }
}
async fn download_file(
    download: github::Download,
    install_path: String,
    progress: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
) -> Result<(), String> {
    let install_dir = utils::expand_tilde(install_path).unwrap();
    let mut temp_dir = utils::expand_tilde(constants::TEMP_DIR).unwrap();
    temp_dir.push(format!("{}.tar.gz", &download.version));

    // install_dir
    fs::create_dir_all(&install_dir).unwrap();

    let git_hash = file::download_file_into_memory(&download.sha512sum)
        .await
        .unwrap();

    if temp_dir.exists() {
        fs::remove_file(&temp_dir);
    }

    file::download_file_progress(
        download.download,
        download.size,
        temp_dir.clone(),
        progress,
        done,
    )
    .await
    .unwrap();
    println!("Checking file integrity");
    if !file::hash_check_file(temp_dir.to_str().unwrap().to_string(), git_hash) {
        return Err("Failed checking file hash".to_string());
    }
    println!("Unpacking files into install location. Please wait");
    file::decompress(temp_dir, install_dir).unwrap();
    return Ok(());
}
