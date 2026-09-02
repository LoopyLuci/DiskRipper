use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use diskripper_core::{
    rip::RipEngine,
    job::JobStatus,
};

#[derive(Parser)]
#[command(name = "diskripper")]
#[command(about = "DiskRipper - Next generation media backup software")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Set log level
    #[arg(short, long, global = true, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    ListDrives,
    DriveInfo { drive_id: String },
    Rip {
        #[arg(short, long)]
        drive: String,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        verify: bool,
        #[arg(long)]
        eject: bool,
    },
    Extract {
        #[arg(short, long)]
        drive: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    Verify {
        #[arg(short, long)]
        drive: String,
        #[arg(short, long)]
        image: PathBuf,
    },
    Info { drive_id: String },
    ListJobs,
    JobStatus { job_id: String },
    CancelJob { job_id: String },
}

fn main() {
    let cli = Cli::parse();
    let log_level = if cli.verbose { "debug" } else { &cli.log_level };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let engine = RipEngine::new();

    match cli.command {
        Commands::ListDrives => cmd_list_drives(&engine),
        Commands::DriveInfo { drive_id } => cmd_drive_info(&engine, drive_id),
        Commands::Rip { drive, output, verify: _, eject } => {
            rt.block_on(cmd_rip(&engine, &drive, &output, eject));
        }
        Commands::Extract { drive, output } => rt.block_on(cmd_extract(&engine, &drive, &output)),
        Commands::Verify { drive, image } => rt.block_on(cmd_verify(&engine, &drive, &image)),
        Commands::Info { drive_id } => cmd_info(&engine, drive_id),
        Commands::ListJobs => cmd_list_jobs(&engine),
        Commands::JobStatus { job_id } => cmd_job_status(&engine, job_id),
        Commands::CancelJob { job_id } => rt.block_on(cmd_cancel_job(&engine, job_id)),
    }
}

fn cmd_list_drives(engine: &RipEngine) {
    println!("Scanning for optical drives...");
    let drives = engine.drives();
    if drives.is_empty() {
        println!("No optical drives detected.");
        return;
    }
    for drive in &drives {
        println!("  {} - {} ({})", drive.id, drive.path, drive.drive_type);
    }
}

fn cmd_drive_info(engine: &RipEngine, drive_id: String) {
    match engine.analyze_drive(&drive_id) {
        Ok(info) => {
            println!("Disc Type:  {}", info.disc_type);
            println!("Size:       {} bytes", info.total_size);
            println!("Filesystem: {}", info.file_system);
            println!("Tracks:     {}", info.tracks);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_rip(engine: &RipEngine, drive_id: &str, output: &Path, eject: bool) {
    println!("Ripping drive {} to {}", drive_id, output.display());
    let options = diskripper_core::types::ImageOptions::default();
    match engine.start_image_rip(drive_id, output, options).await {
        Ok(job_id) => {
            println!("Job started: {}", job_id);
            wait_for_job(engine, &job_id.0).await;
            if eject { println!("Eject requested"); }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_extract(engine: &RipEngine, drive_id: &str, output: &Path) {
    println!("Extracting from drive {} to {}", drive_id, output.display());
    let options = diskripper_core::types::ExtractOptions {
        preserve_timestamps: true,
        preserve_permissions: true,
        overwrite_existing: false,
        extract_path: Some(output.to_path_buf()),
    };
    match engine.start_extraction(drive_id, output, options).await {
        Ok(job_id) => {
            println!("Job started: {}", job_id);
            wait_for_job(engine, &job_id.0).await;
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_verify(engine: &RipEngine, drive_id: &str, image: &PathBuf) {
    println!("Verifying {} against drive {}", image.display(), drive_id);
    let drives = engine.drives();
    let drive = match drives.iter().find(|d| d.id == drive_id) {
        Some(d) => d,
        None => {
            eprintln!("Drive not found");
            std::process::exit(1);
        }
    };
    let metadata = std::fs::metadata(image).expect("Cannot read image");
    let total_size = metadata.len();
    let job_id = engine.job_manager().create_job("CLI Verify".to_string());
    let _ = engine.job_manager().set_status(&job_id, JobStatus::Running);
    match diskripper_core::filesystem::verify::verify_disc_image(
        engine.job_manager(),
        job_id,
        std::path::Path::new(&drive.path),
        image.as_path(),
        total_size,
    ) {
        Ok(results) => {
            let passed = results.iter().filter(|r| r.valid).count();
            println!("Passed: {}/{}", passed, results.len());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_info(engine: &RipEngine, drive_id: String) {
    cmd_drive_info(engine, drive_id);
}

fn cmd_list_jobs(engine: &RipEngine) {
    let jobs = engine.job_manager().list_jobs();
    if jobs.is_empty() {
        println!("No jobs found.");
        return;
    }
    for job in &jobs {
        println!("  {} | {} | {:.1}%", job.id, job.name, job.progress.percent());
    }
}

fn cmd_job_status(engine: &RipEngine, job_id: String) {
    let id = diskripper_core::job::JobId(job_id);
    match engine.job_manager().get_job(&id) {
        Some(job) => {
            println!("Status: {}", job.status);
            println!("Progress: {:.1}%", job.progress.percent());
            if let Some(err) = &job.error {
                println!("Error: {}", err);
            }
        }
        None => {
            eprintln!("Job not found");
            std::process::exit(1);
        }
    }
}

async fn cmd_cancel_job(engine: &RipEngine, job_id: String) {
    let id = diskripper_core::job::JobId(job_id);
    match engine.cancel_job(&id).await {
        Ok(_) => println!("Job cancelled"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn wait_for_job(engine: &RipEngine, job_id: &str) {
    let id = diskripper_core::job::JobId(job_id.to_string());
    loop {
        if let Some(job) = engine.job_manager().get_job(&id) {
            match job.status {
                JobStatus::Completed => { println!("Completed!"); return; }
                JobStatus::Failed => {
                    eprintln!("Failed: {}", job.error.as_deref().unwrap_or("Unknown"));
                    std::process::exit(1);
                }
                JobStatus::Cancelled => {
                    println!("Cancelled");
                    std::process::exit(1);
                }
                _ => {
                    print!("\rProgress: {:.1}%", job.progress.percent());
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}
