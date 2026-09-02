use tauri::State;
use tracing::info;

use diskripper_core::disc::DiscInfo;
use diskripper_core::drive::DriveInfo;
use diskripper_core::filesystem::verify::VerificationResult;
use diskripper_core::job::Job;
use diskripper_core::rip::RipEngine;
use diskripper_core::settings::Settings;
use diskripper_core::types::*;

#[tauri::command]
fn list_drives(engine: State<'_, RipEngine>) -> Result<Vec<DriveInfo>, String> {
    Ok(engine.drives())
}

#[tauri::command]
fn list_jobs(engine: State<'_, RipEngine>) -> Result<Vec<Job>, String> {
    Ok(engine.job_manager().list_jobs())
}

#[tauri::command]
fn start_image_rip(
    drive_id: String,
    output_path: String,
    engine: State<'_, RipEngine>,
) -> Result<String, String> {
    info!(drive_id = %drive_id, output = %output_path, "Starting image rip");
    let opts = ImageOptions::default();
    let path = std::path::PathBuf::from(output_path);
    tauri::async_runtime::block_on(engine.start_image_rip(&drive_id, &path, opts))
        .map(|id| id.0)
        .map_err(|e| format!("Failed to start image rip: {}", e))
}

#[tauri::command]
fn start_extraction(
    drive_id: String,
    output_path: String,
    engine: State<'_, RipEngine>,
) -> Result<String, String> {
    info!(drive_id = %drive_id, output = %output_path, "Starting extraction");
    let opts = ExtractOptions::default();
    let path = std::path::PathBuf::from(output_path);
    tauri::async_runtime::block_on(engine.start_extraction(&drive_id, &path, opts))
        .map(|id| id.0)
        .map_err(|e| format!("Failed to start extraction: {}", e))
}

#[tauri::command]
fn cancel_job(job_id: String, engine: State<'_, RipEngine>) -> Result<(), String> {
    let id = JobId(job_id);
    tauri::async_runtime::block_on(engine.cancel_job(&id))
        .map_err(|e| format!("Failed to cancel job: {}", e))
}

#[tauri::command]
fn get_job(job_id: String, engine: State<'_, RipEngine>) -> Result<Job, String> {
    let id = JobId(job_id);
    engine
        .job_manager()
        .get_job(&id)
        .ok_or_else(|| "Job not found".to_string())
}

#[tauri::command]
fn remove_job(job_id: String, engine: State<'_, RipEngine>) -> Result<(), String> {
    let id = JobId(job_id);
    engine
        .job_manager()
        .remove_job(&id)
        .map_err(|e| format!("Failed to remove job: {}", e))
}

#[tauri::command]
fn analyze_drive(drive_id: String, engine: State<'_, RipEngine>) -> Result<DiscInfo, String> {
    engine
        .analyze_drive(&drive_id)
        .map_err(|e| format!("Failed to analyze drive: {}", e))
}

#[tauri::command]
fn get_default_output_path() -> String {
    let home = dirs::home_dir().unwrap_or_default();
    home.join("DiskRipper").to_string_lossy().to_string()
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_app_dir() -> String {
    std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
fn load_settings(state: State<'_, SettingsState>) -> Result<Settings, String> {
    Ok(state.inner().0.lock().unwrap().get())
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, SettingsState>) -> Result<(), String> {
    state
        .inner()
        .0
        .lock()
        .unwrap()
        .update(settings)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
fn reset_settings(state: State<'_, SettingsState>) -> Result<Settings, String> {
    let manager = &state.inner().0;
    manager
        .lock()
        .unwrap()
        .reset_to_defaults()
        .map_err(|e| format!("Failed to reset settings: {}", e))?;
    Ok(manager.lock().unwrap().get())
}

#[tauri::command]
fn verify_image_rip(
    drive_id: String,
    image_path: String,
    engine: State<'_, RipEngine>,
) -> Result<Vec<VerificationResult>, String> {
    use diskripper_core::filesystem::verify::verify_disc_image;

    let drive = engine
        .drives()
        .into_iter()
        .find(|d| d.id == drive_id)
        .ok_or("Drive not found")?;

    let metadata =
        std::fs::metadata(&image_path).map_err(|e| format!("Failed to read image: {}", e))?;
    let total_size = metadata.len();

    let job_id = engine.job_manager().create_job("Verify image".to_string());
    engine
        .job_manager()
        .set_status(&job_id, diskripper_core::job::JobStatus::Running)
        .map_err(|e| e.to_string())?;

    let result = verify_disc_image(
        engine.job_manager(),
        job_id.clone(),
        std::path::Path::new(&drive.path),
        std::path::Path::new(&image_path),
        total_size,
    )
    .map_err(|e| format!("Verification failed: {}", e))?;

    engine
        .job_manager()
        .set_status(&job_id, diskripper_core::job::JobStatus::Completed)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
fn get_audio_tracks(
    drive_id: String,
    engine: State<'_, RipEngine>,
) -> Result<Vec<serde_json::Value>, String> {
    use diskripper_core::audio::AudioCdReader;

    let drive = engine
        .drives()
        .into_iter()
        .find(|d| d.id == drive_id)
        .ok_or("Drive not found")?;

    let mut reader = AudioCdReader::new(drive.path, 0);
    let tracks = reader
        .parse_toc()
        .map_err(|e| format!("Failed to parse TOC: {}", e))?;

    let result: Vec<serde_json::Value> = tracks
        .iter()
        .map(|t| {
            serde_json::json!({
                "track_number": t.track_number,
                "start_sector": t.start_sector,
                "end_sector": t.end_sector,
                "duration_seconds": t.duration_seconds,
                "is_audio": t.is_audio,
            })
        })
        .collect();

    Ok(result)
}

#[tauri::command]
fn extract_audio_track_to_wav(
    drive_id: String,
    track_number: u8,
    output_path: String,
    engine: State<'_, RipEngine>,
) -> Result<String, String> {
    use diskripper_core::audio::{extract_audio_track, AudioCdReader};

    let drive = engine
        .drives()
        .into_iter()
        .find(|d| d.id == drive_id)
        .ok_or("Drive not found")?;

    let mut reader = AudioCdReader::new(drive.path.clone(), 0);
    let tracks = reader
        .parse_toc()
        .map_err(|e| format!("Failed to parse TOC: {}", e))?;

    let track = tracks
        .iter()
        .find(|t| t.track_number == track_number)
        .ok_or("Track not found")?;

    let num_sectors = track.sector_count();

    extract_audio_track(
        std::path::Path::new(&drive.path),
        track.start_sector,
        num_sectors,
        std::path::Path::new(&output_path),
        true,
    )
    .map_err(|e| format!("Failed to extract track: {}", e))?;

    Ok(format!(
        "Track {} extracted to {}",
        track_number, output_path
    ))
}

pub struct SettingsState(std::sync::Mutex<diskripper_core::settings::SettingsManager>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize file logging
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("DiskRipper")
        .join("logs");

    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create log directory: {}", e);
    }

    let file_appender = tracing_appender::rolling::daily(&log_dir, "diskripper.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    // Load settings
    let app_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("DiskRipper");

    if let Err(e) = std::fs::create_dir_all(&app_dir) {
        tracing::error!("Failed to create app directory: {}", e);
    }

    let settings_manager =
        diskripper_core::settings::SettingsManager::new(app_dir).unwrap_or_else(|e| {
            tracing::error!("Failed to load settings: {}, using defaults", e);
            diskripper_core::settings::SettingsManager::new(std::env::temp_dir()).unwrap()
        });

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(RipEngine::new())
        .manage(SettingsState(std::sync::Mutex::new(settings_manager)))
        .invoke_handler(tauri::generate_handler![
            list_drives,
            list_jobs,
            start_image_rip,
            start_extraction,
            cancel_job,
            get_job,
            remove_job,
            analyze_drive,
            get_default_output_path,
            get_version,
            get_app_dir,
            load_settings,
            save_settings,
            reset_settings,
            verify_image_rip,
            get_audio_tracks,
            extract_audio_track_to_wav,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
