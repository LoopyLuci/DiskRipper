use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use eframe::egui;

use diskripper_core::audio::{extract_audio_track, AudioCdReader, TrackInfo};
use diskripper_core::drive::DriveInfo;
use diskripper_core::disc::DiscInfo;
use diskripper_core::job::{Job, JobStatus};
use diskripper_core::rip::RipEngine;
use diskripper_core::settings::{Settings, SettingsManager};
use diskripper_core::types::{CompressionLevel, ImageFormat, ImageOptions};
use diskripper_core::ml::pipeline::{ContentType, IdentificationSource, PipelineResult};
use diskripper_core::filesystem::verify::verify_file_rip;

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
    TracksLoaded(Vec<TrackInfo>),
    TrackExtracted(String),
    VerificationDone { path: String, valid: bool, message: String },
    Identified(PipelineResult),
    Status(String),
    Error(String),
}

#[derive(Debug)]
enum UiCommand {
    RefreshDrives,
    AnalyzeDrive(String),
    StartRip { drive_id: String, output_path: String },
    CancelJob(String),
    LoadTracks { drive_path: String },
    ExtractTrack { drive_path: String, track: TrackInfo, output_path: String },
    IdentifyAudio { file_path: String },
    VerifyFile { drive_path: String, file_path: String },
}

pub struct DiskRipperApp {
    engine: Arc<RipEngine>,
    settings_manager: Arc<SettingsManager>,
    settings: Settings,
    drives: Vec<DriveInfo>,
    selected_drive: Option<usize>,
    disc_info: Option<DiscInfo>,
    jobs: Vec<Job>,
    tracks: Vec<TrackInfo>,
    current_tab: Tab,
    status_message: String,
    output_path: String,
    ml_file_path: String,
    ml_result: Option<PipelineResult>,
    verification_message: String,
    cmd_tx: mpsc::Sender<UiCommand>,
    event_rx: mpsc::Receiver<UiEvent>,
}

impl DiskRipperApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let engine = Arc::new(RipEngine::new());
        let drives = engine.drives();

        // Load settings
        let app_dir = std::env::var("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("DiskRipper");
        let settings_manager = Arc::new(
            SettingsManager::new(app_dir).unwrap_or_else(|_| {
                // Fallback: create with temp dir
                let tmp = std::env::temp_dir().join("diskripper");
                let _ = std::fs::create_dir_all(&tmp);
                SettingsManager::new(tmp).expect("Failed to create settings manager")
            }),
        );
        let settings = settings_manager.get();

        let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>();
        let (event_tx, event_rx) = mpsc::channel::<UiEvent>();

        let engine_clone = engine.clone();
        thread::spawn(move || {
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    UiCommand::RefreshDrives => {
                        let drives = engine_clone.drives();
                        let _ = event_tx.send(UiEvent::DrivesLoaded(drives));
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
                    UiCommand::LoadTracks { drive_path } => {
                        let _ = event_tx.send(UiEvent::Status("Loading tracks...".into()));
                        let tx = event_tx.clone();
                        thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                // Get disc size first
                                let total_sectors = match diskripper_core::filesystem::reader::get_disc_size(&drive_path) {
                                    Ok(size) => size / 2352,
                                    Err(_) => 337500, // ~80 min CD default
                                };
                                let mut reader = AudioCdReader::new(drive_path, total_sectors);
                                match reader.parse_toc() {
                                    Ok(tracks) => {
                                        let _ = tx.send(UiEvent::TracksLoaded(tracks));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(UiEvent::Error(e.to_string()));
                                    }
                                }
                            });
                        });
                    }
                    UiCommand::ExtractTrack { drive_path, track, output_path } => {
                        let _ = event_tx.send(UiEvent::Status(format!("Extracting track {}...", track.track_number)));
                        let tx = event_tx.clone();
                        thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                let path = std::path::PathBuf::from(&output_path);
                                let result = extract_audio_track(
                                    std::path::Path::new(&drive_path),
                                    track.start_sector,
                                    track.sector_count(),
                                    &path,
                                    true,
                                );
                                match result {
                                    Ok(_) => {
                                        let _ = tx.send(UiEvent::TrackExtracted(output_path));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(UiEvent::Error(e.to_string()));
                                    }
                                }
                            });
                        });
                    }
                    UiCommand::IdentifyAudio { file_path } => {
                        let _ = event_tx.send(UiEvent::Status("Identifying audio...".into()));
                        let engine = engine_clone.clone();
                        let tx = event_tx.clone();
                        thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                match std::fs::read(&file_path) {
                                    Ok(data) => {
                                        // Parse WAV header to extract PCM samples
                                        let samples = parse_wav_samples(&data);
                                        if samples.is_empty() {
                                            let _ = tx.send(UiEvent::Error("Could not parse WAV file or file is empty".into()));
                                            return;
                                        }
                                        match engine.identify_audio(&samples, 44100) {
                                            Ok(result) => {
                                                let _ = tx.send(UiEvent::Identified(result));
                                            }
                                            Err(e) => {
                                                let _ = tx.send(UiEvent::Error(e.to_string()));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(UiEvent::Error(e.to_string()));
                                    }
                                }
                            });
                        });
                    }
                    UiCommand::VerifyFile { drive_path, file_path } => {
                        let _ = event_tx.send(UiEvent::Status("Verifying...".into()));
                        let tx = event_tx.clone();
                        thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                let drive = std::path::PathBuf::from(&drive_path);
                                let file = std::path::PathBuf::from(&file_path);
                                match verify_file_rip(&drive, &file, 0, 0, 2048) {
                                    Ok(result) => {
                                        let msg = if result.valid {
                                            format!("Verification PASSED ({} bytes verified)", result.bytes_verified)
                                        } else {
                                            format!("Verification FAILED - checksums don't match")
                                        };
                                        let _ = tx.send(UiEvent::VerificationDone {
                                            path: result.file_path,
                                            valid: result.valid,
                                            message: msg,
                                        });
                                    }
                                    Err(e) => {
                                        let _ = tx.send(UiEvent::Error(e.to_string()));
                                    }
                                }
                            });
                        });
                    }
                }
            }
        });

        let output_path = settings.default_output_dir.to_string_lossy().to_string();

        Self {
            engine,
            settings_manager,
            settings,
            drives,
            selected_drive: None,
            disc_info: None,
            jobs: Vec::new(),
            tracks: Vec::new(),
            current_tab: Tab::Drives,
            status_message: "Ready".into(),
            output_path,
            ml_file_path: String::new(),
            ml_result: None,
            verification_message: String::new(),
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
                UiEvent::TracksLoaded(tracks) => {
                    self.tracks = tracks;
                    self.status_message = format!("Loaded {} tracks", self.tracks.len());
                }
                UiEvent::TrackExtracted(path) => {
                    self.status_message = format!("Extracted to {}", path);
                }
                UiEvent::VerificationDone { message, .. } => {
                    self.verification_message = message;
                    self.status_message = "Verification complete".into();
                }
                UiEvent::Identified(result) => {
                    self.ml_result = Some(result);
                    self.status_message = "Identification complete".into();
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
                let drive = self.drives[idx].clone();
                ui.label(format!("Drive: {}", drive.path));
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Load Tracks").clicked() {
                        let _ = self.cmd_tx.send(UiCommand::LoadTracks {
                            drive_path: drive.path.clone(),
                        });
                    }
                });

                ui.separator();

                if !self.tracks.is_empty() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for track in &self.tracks {
                            egui::Frame::window(ui.style()).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(format!("Track {:02}", track.track_number));
                                    let mins = (track.duration_seconds / 60.0) as u32;
                            let secs = (track.duration_seconds % 60.0) as u32;
                            ui.label(format!("{}:{:02}", mins, secs));
                                    if !track.is_audio {
                                        ui.label(egui::RichText::new("(data)").color(egui::Color32::GRAY));
                                    }
                                    if track.is_audio && ui.button("Extract").clicked() {
                                        let out = format!("{}/track{:02}.wav", self.output_path, track.track_number);
                                        let _ = self.cmd_tx.send(UiCommand::ExtractTrack {
                                            drive_path: drive.path.clone(),
                                            track: track.clone(),
                                            output_path: out,
                                        });
                                    }
                                });
                            });
                        }
                    });
                } else {
                    ui.label("Click 'Load Tracks' to read the TOC from the disc.");
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Output:");
                    ui.text_edit_singleline(&mut self.output_path);
                });
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
                let drive = self.drives[idx].clone();
                ui.label(format!("Drive: {}", drive.path));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Image file:");
                    ui.text_edit_singleline(&mut self.output_path);
                });

                ui.separator();

                if ui.button("Verify").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::VerifyFile {
                        drive_path: drive.path.clone(),
                        file_path: self.output_path.clone(),
                    });
                }

                if !self.verification_message.is_empty() {
                    ui.separator();
                    let color = if self.verification_message.contains("PASSED") {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };
                    ui.label(egui::RichText::new(&self.verification_message).color(color));
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
                ui.label("Audio file (WAV):");
                ui.text_edit_singleline(&mut self.ml_file_path);
            });

            ui.separator();

            if ui.button("Identify Content").clicked() && !self.ml_file_path.is_empty() {
                let _ = self.cmd_tx.send(UiCommand::IdentifyAudio {
                    file_path: self.ml_file_path.clone(),
                });
            }

            if let Some(result) = &self.ml_result {
                ui.separator();
                ui.heading("Results");
                ui.label(format!("Content type: {:?}", result.content_type));
                ui.label(format!("Confidence: {:.1}%", result.confidence * 100.0));
                if let Some(title) = &result.title {
                    ui.label(format!("Title: {}", title));
                }
                if let Some(artist) = &result.artist {
                    ui.label(format!("Artist: {}", artist));
                }
                if let Some(album) = &result.album {
                    ui.label(format!("Album: {}", album));
                }
                if let Some(genre) = &result.genre {
                    ui.label(format!("Genre: {}", genre));
                }
                if let Some(year) = &result.year {
                    ui.label(format!("Year: {}", year));
                }
                ui.label(format!("Source: {:?}", result.source));
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
                    ui.label("Read retries:");
                    ui.add(egui::DragValue::new(&mut self.settings.read_retries).range(0..=10));
                });
                ui.horizontal(|ui| {
                    ui.label("Buffer size (MB):");
                    ui.add(egui::DragValue::new(&mut self.settings.buffer_size_mb).range(1..=64));
                });
            });

            ui.collapsing("Options", |ui| {
                ui.checkbox(&mut self.settings.verify_checksums, "Verify checksums");
                ui.checkbox(&mut self.settings.jitter_correction, "Jitter correction");
                ui.checkbox(&mut self.settings.eject_after_rip, "Eject after rip");
            });

            ui.collapsing("Processing", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Max concurrent jobs:");
                    ui.add(egui::DragValue::new(&mut self.settings.max_concurrent_jobs).range(1..=8));
                });
                ui.checkbox(&mut self.settings.enable_parallel, "Enable parallel processing");
            });

            ui.separator();
            if ui.button("Save Settings").clicked() {
                self.settings.default_output_dir = std::path::PathBuf::from(&self.output_path);
                match self.settings_manager.update(self.settings.clone()) {
                    Ok(_) => {
                        self.status_message = "Settings saved".into();
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save settings: {}", e);
                    }
                }
            }
        });
    }
}

impl eframe::App for DiskRipperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();

        if self.settings.theme == "light" {
            ctx.set_visuals(egui::Visuals::light());
        } else {
            ctx.set_visuals(egui::Visuals::dark());
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

/// Parse a WAV file and extract PCM i16 samples
fn parse_wav_samples(data: &[u8]) -> Vec<i16> {
    if data.len() < 44 {
        return Vec::new();
    }
    // Check RIFF/WAVE headers
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Vec::new();
    }
    // Find the data chunk
    let mut pos = 12;
    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        if chunk_id == b"data" {
            let data_start = pos + 8;
            let data_end = std::cmp::min(data_start + chunk_size, data.len());
            let mut samples = Vec::new();
            let mut i = data_start;
            while i + 2 <= data_end {
                let sample = i16::from_le_bytes([data[i], data[i + 1]]);
                samples.push(sample);
                i += 2;
            }
            return samples;
        }
        pos += 8 + chunk_size;
    }
    Vec::new()
}
