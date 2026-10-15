use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use eframe::egui;

use diskripper_core::drive::DriveInfo;
use diskripper_core::disc::DiscInfo;
use diskripper_core::job::{Job, JobStatus};
use diskripper_core::rip::RipEngine;
use diskripper_core::types::{CompressionLevel, ImageFormat, ImageOptions};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Drives,
    Jobs,
    Audio,
    Verify,
    ML,
    Settings,
}

#[derive(Debug)]
enum UiEvent {
    DrivesLoaded(Vec<DriveInfo>),
    DiscAnalyzed(DiscInfo),
    JobStarted(String),
    JobUpdated(Job),
    JobCompleted(String),
    Status(String),
    Error(String),
}

#[derive(Debug)]
enum UiCommand {
    RefreshDrives,
    AnalyzeDrive(String),
    StartRip { drive_id: String, output_path: String },
    CancelJob(String),
    LoadAudioTracks(String),
    ExtractAudio { drive_id: String, output_path: String },
    IdentifyAudio { file_path: String },
    VerifyImage { image_path: String, drive_id: String },
}

pub struct DiskRipperApp {
    engine: Arc<RipEngine>,
    drives: Vec<DriveInfo>,
    selected_drive: Option<usize>,
    disc_info: Option<DiscInfo>,
    jobs: Vec<Job>,
    current_tab: Tab,
    status_message: String,
    output_path: String,
    read_speed: u32,
    verify_checksums: bool,
    jitter_correction: bool,
    dark_mode: bool,
    image_format: ImageFormat,
    compression: CompressionLevel,
    cmd_tx: mpsc::Sender<UiCommand>,
    event_rx: mpsc::Receiver<UiEvent>,
}

impl DiskRipperApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let engine = Arc::new(RipEngine::new());
        let drives = engine.drives();

        let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>();
        let (event_tx, event_rx) = mpsc::channel::<UiEvent>();

        let engine_clone = engine.clone();
        thread::spawn(move || {
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    UiCommand::RefreshDrives => {
                        let _ = event_tx.send(UiEvent::Status("Refreshing drives...".into()));
                    }
                    UiCommand::AnalyzeDrive(drive_id) => {
                        let _ = event_tx.send(UiEvent::Status("Analyzing disc...".into()));
                        match engine_clone.analyze_drive(&drive_id) {
                            Ok(info) => {
                                let _ = event_tx.send(UiEvent::DiscAnalyzed(info));
                            }
                            Err(e) => {
                                let _ = event_tx.send(UiEvent::Error(e.to_string()));
                            }
                        }
                    }
                    UiCommand::StartRip { drive_id, output_path } => {
                        let _ = event_tx.send(UiEvent::Status("Starting rip...".into()));
                        let path = std::path::PathBuf::from(&output_path);
                        let options = ImageOptions {
                            format: ImageFormat::Iso,
                            compression: CompressionLevel::None,
                            split_size: None,
                        };
                        let engine = engine_clone.clone();
                        let tx = event_tx.clone();
                        thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                match engine.start_image_rip(&drive_id, &path, options).await {
                                    Ok(job_id) => {
                                        let _ = tx.send(UiEvent::JobStarted(job_id.to_string()));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(UiEvent::Error(e.to_string()));
                                    }
                                }
                            });
                        });
                    }
                    UiCommand::CancelJob(_job_id) => {
                        let _ = event_tx.send(UiEvent::Status("Cancelling...".into()));
                    }
                    UiCommand::LoadAudioTracks(_drive_id) => {
                        let _ = event_tx.send(UiEvent::Status("Loading tracks...".into()));
                    }
                    UiCommand::ExtractAudio { .. } => {
                        let _ = event_tx.send(UiEvent::Status("Extracting audio...".into()));
                    }
                    UiCommand::IdentifyAudio { .. } => {
                        let _ = event_tx.send(UiEvent::Status("Identifying...".into()));
                    }
                    UiCommand::VerifyImage { .. } => {
                        let _ = event_tx.send(UiEvent::Status("Verifying...".into()));
                    }
                }
            }
        });

        Self {
            engine,
            drives,
            selected_drive: None,
            disc_info: None,
            jobs: Vec::new(),
            current_tab: Tab::Drives,
            status_message: "Ready".into(),
            output_path: "C:\\DiskRipper".into(),
            read_speed: 0,
            verify_checksums: true,
            jitter_correction: true,
            dark_mode: true,
            image_format: ImageFormat::Iso,
            compression: CompressionLevel::None,
            cmd_tx,
            event_rx,
        }
    }

    fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                UiEvent::DrivesLoaded(drives) => {
                    self.drives = drives;
                    self.status_message = "Drives refreshed".into();
                }
                UiEvent::DiscAnalyzed(info) => {
                    self.disc_info = Some(info);
                    self.status_message = "Disc analyzed".into();
                }
                UiEvent::JobStarted(job_id) => {
                    self.status_message = format!("Job {} started", job_id);
                    self.current_tab = Tab::Jobs;
                }
                UiEvent::JobUpdated(job) => {
                    if let Some(existing) = self.jobs.iter_mut().find(|j| j.id == job.id) {
                        *existing = job;
                    } else {
                        self.jobs.push(job);
                    }
                }
                UiEvent::JobCompleted(job_id) => {
                    self.status_message = format!("Job {} completed", job_id);
                }
                UiEvent::Status(msg) => {
                    self.status_message = msg;
                }
                UiEvent::Error(err) => {
                    self.status_message = format!("Error: {}", err);
                }
            }
        }
    }

    fn render_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side_panel")
            .max_width(180.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("DiskRipper");
                    ui.separator();

                    let tabs = [
                        (Tab::Drives, "Drives"),
                        (Tab::Jobs, "Jobs"),
                        (Tab::Audio, "Audio CD"),
                        (Tab::Verify, "Verify"),
                        (Tab::ML, "ML Identify"),
                        (Tab::Settings, "Settings"),
                    ];

                    for (tab, label) in &tabs {
                        let active = self.current_tab == *tab;
                        let response = ui.selectable_label(active, *label);
                        if response.clicked() {
                            self.current_tab = *tab;
                        }
                    }
                });
            });
    }

    fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel")
            .min_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);
                });
            });
    }

    fn render_drives_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Optical Drives");
            ui.separator();

            if ui.button("Refresh Drives").clicked() {
                let _ = self.cmd_tx.send(UiCommand::RefreshDrives);
            }

            ui.separator();

            if self.drives.is_empty() {
                ui.label(
                    egui::RichText::new("No optical drives detected.")
                        .size(16.0)
                        .color(egui::Color32::YELLOW),
                );
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, drive) in self.drives.iter().enumerate() {
                    let selected = self.selected_drive == Some(i);
                    let frame = egui::Frame::window(ui.style()).fill(if selected {
                        ui.visuals().extreme_bg_color
                    } else {
                        ui.visuals().faint_bg_color
                    });

                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("💾").size(24.0));
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&drive.path).size(16.0).strong());
                                ui.label(egui::RichText::new(format!("{}", drive.drive_type)).size(12.0).color(egui::Color32::GRAY));
                                if let Some(label) = &drive.label {
                                    ui.label(egui::RichText::new(label).size(12.0));
                                }
                            });
                        });
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Select").clicked() {
                                self.selected_drive = Some(i);
                            }
                            if ui.button("Analyze").clicked() {
                                let _ = self.cmd_tx.send(UiCommand::AnalyzeDrive(drive.id.clone()));
                            }
                            if ui.button("Rip").clicked() {
                                let output = format!("{}/{}.iso", self.output_path, drive.path.replace(':', ""));
                                let _ = self.cmd_tx.send(UiCommand::StartRip {
                                    drive_id: drive.id.clone(),
                                    output_path: output,
                                });
                            }
                        });
                    });
                }
            });

            if let Some(info) = &self.disc_info {
                ui.separator();
                ui.heading("Disc Information");
                ui.label(format!("Type: {:?}", info.disc_type));
                ui.label(format!("Label: {}", info.label.as_deref().unwrap_or("N/A")));
                ui.label(format!("Total: {} MB", info.total_size / 1_000_000));
                ui.label(format!("Used: {} MB", info.used_size / 1_000_000));
                ui.label(format!("Sessions: {}", info.sessions));
                ui.label(format!("Tracks: {}", info.tracks));
            }
        });
    }

    fn render_jobs_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Jobs");
            ui.separator();

            if self.jobs.is_empty() {
                ui.label(
                    egui::RichText::new("No jobs yet. Start a rip from the Drives tab.")
                        .size(14.0)
                        .color(egui::Color32::GRAY),
                );
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for job in &self.jobs {
                    let status_color = match job.status {
                        JobStatus::Completed => egui::Color32::GREEN,
                        JobStatus::Failed => egui::Color32::RED,
                        JobStatus::Running => egui::Color32::BLUE,
                        JobStatus::Cancelled => egui::Color32::YELLOW,
                        _ => egui::Color32::WHITE,
                    };

                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&job.id.to_string()).size(12.0).monospace());
                            ui.label(egui::RichText::new(job.name.clone()).size(14.0));
                            ui.label(
                                egui::RichText::new(format!("{}", job.status))
                                    .size(14.0)
                                    .color(status_color),
                            );
                        });
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(format!("{:.1}%", job.progress.percent()));
                            ui.add(
                                egui::ProgressBar::new(job.progress.percent() as f32 / 100.0)
                                    .show_percentage(),
                            );
                        });
                        if let Some(err) = &job.error {
                            ui.label(egui::RichText::new(err).color(egui::Color32::RED));
                        }
                    });
                    ui.separator();
                }
            });
        });
    }

    fn render_audio_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Audio CD Extraction");
            ui.separator();

            if let Some(idx) = self.selected_drive {
                let drive = &self.drives[idx];
                ui.label(format!("Drive: {}", drive.path));
                ui.separator();

                if ui.button("Load Tracks").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::LoadAudioTracks(drive.id.clone()));
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Output:");
                    ui.text_edit_singleline(&mut self.output_path);
                });

                ui.separator();

                if ui.button("Extract All").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::ExtractAudio {
                        drive_id: drive.id.clone(),
                        output_path: self.output_path.clone(),
                    });
                }
            } else {
                ui.label(egui::RichText::new("Select a drive first.").color(egui::Color32::YELLOW));
            }
        });
    }

    fn render_verify_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Verify Image");
            ui.separator();

            if let Some(idx) = self.selected_drive {
                let drive = &self.drives[idx];
                ui.label(format!("Drive: {}", drive.path));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Image file:");
                    ui.text_edit_singleline(&mut self.output_path);
                });

                ui.separator();

                if ui.button("Verify").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::VerifyImage {
                        image_path: self.output_path.clone(),
                        drive_id: drive.id.clone(),
                    });
                }
            } else {
                ui.label(egui::RichText::new("Select a drive first.").color(egui::Color32::YELLOW));
            }
        });
    }

    fn render_ml_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ML Content Identification");
            ui.separator();

            ui.label("Identify content using machine learning.");
            ui.label("All processing is local — no external API dependencies.");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Audio file:");
                ui.text_edit_singleline(&mut self.output_path);
            });

            ui.separator();

            if ui.button("Identify Content").clicked() {
                let _ = self.cmd_tx.send(UiCommand::IdentifyAudio {
                    file_path: self.output_path.clone(),
                });
            }

            ui.separator();

            ui.collapsing("How It Works", |ui| {
                ui.label("• Custom audio fingerprinting identifies music from actual audio");
                ui.label("• Hybrid ML combines fingerprinting + classification + metadata");
                ui.label("• Self-learning pipeline improves from your feedback");
            });
        });
    }

    fn render_settings_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Settings");
            ui.separator();

            ui.collapsing("Output", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Output directory:");
                    ui.text_edit_singleline(&mut self.output_path);
                });
            });

            ui.collapsing("Reading", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Read speed:");
                    let mut speed = self.read_speed;
                    ui.radio_value(&mut speed, 0, "Maximum");
                    ui.radio_value(&mut speed, 16, "16x");
                    ui.radio_value(&mut speed, 8, "8x");
                    self.read_speed = speed;
                });
            });

            ui.collapsing("Options", |ui| {
                ui.checkbox(&mut self.verify_checksums, "Verify checksums");
                ui.checkbox(&mut self.jitter_correction, "Jitter correction");
            });

            ui.collapsing("Image Format", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    ui.radio_value(&mut self.image_format, ImageFormat::Iso, "ISO");
                    ui.radio_value(&mut self.image_format, ImageFormat::BinCue, "BIN/CUE");
                    ui.radio_value(&mut self.image_format, ImageFormat::Img, "IMG");
                    ui.radio_value(&mut self.image_format, ImageFormat::Dmg, "DMG");
                });
            });

            ui.collapsing("Theme", |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.dark_mode, true, "Dark");
                    ui.radio_value(&mut self.dark_mode, false, "Light");
                });
            });
        });
    }
}

impl eframe::App for DiskRipperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();

        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        self.render_side_panel(ctx);
        self.render_top_panel(ctx);

        match self.current_tab {
            Tab::Drives => self.render_drives_tab(ctx),
            Tab::Jobs => self.render_jobs_tab(ctx),
            Tab::Audio => self.render_audio_tab(ctx),
            Tab::Verify => self.render_verify_tab(ctx),
            Tab::ML => self.render_ml_tab(ctx),
            Tab::Settings => self.render_settings_tab(ctx),
        }
    }
}
