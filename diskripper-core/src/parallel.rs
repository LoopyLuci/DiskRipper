//! Parallel processing and hardware acceleration module.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;
use tracing::info;

use crate::error::DiskRipperError;
use crate::filesystem::FilesystemReader;
use crate::types::*;

/// Hardware system information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub num_cpus: usize,
    pub num_physical_cpus: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub gpu_devices: Vec<GpuDevice>,
}

/// GPU device information
#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub name: String,
    pub vendor: String,
    pub memory_bytes: u64,
    pub platform: String,
}

/// Hardware resource manager with thread pool
pub struct HardwareManager {
    pub system_info: SystemInfo,
    pub thread_pool: rayon::ThreadPool,
}

impl HardwareManager {
    /// Detect system hardware and initialize thread pool
    pub fn detect() -> Self {
        let num_cpus = num_cpus::get();
        let num_physical_cpus = num_cpus::get_physical();
        let (total_memory, available_memory) = Self::detect_memory();
        let gpu_devices = Self::detect_gpus();

        let system_info = SystemInfo {
            num_cpus,
            num_physical_cpus,
            total_memory_bytes: total_memory,
            available_memory_bytes: available_memory,
            gpu_devices,
        };

        // Create rayon thread pool sized to CPU cores
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus)
            .thread_name(|idx| format!("diskripper-worker-{}", idx))
            .build()
            .expect("Failed to create rayon thread pool");

        info!(
            cpus = num_cpus,
            physical_cpus = num_physical_cpus,
            total_memory_gb = total_memory / (1024 * 1024 * 1024),
            gpus = system_info.gpu_devices.len(),
            "Hardware detection complete"
        );

        Self { system_info, thread_pool }
    }

    /// Detect system memory (cross-platform)
    #[cfg(target_os = "windows")]
    fn detect_memory() -> (u64, u64) {
        unsafe {
            let mut mem_info: windows_sys::Win32::System::SystemInformation::MEMORYSTATUSEX =
                std::mem::zeroed();
            mem_info.dwLength = std::mem::size_of::<
                windows_sys::Win32::System::SystemInformation::MEMORYSTATUSEX,
            >() as u32;
            windows_sys::Win32::System::SystemInformation::GlobalMemoryStatusEx(&mut mem_info);
            (mem_info.ullTotalPhys, mem_info.ullAvailPhys)
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_memory() -> (u64, u64) {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut available = 0u64;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    total = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0)
                        * 1024;
                } else if line.starts_with("MemAvailable:") {
                    available = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0)
                        * 1024;
                }
            }
            if total > 0 {
                return (total, available);
            }
        }
        (0, 0)
    }

    #[cfg(target_os = "macos")]
    fn detect_memory() -> (u64, u64) {
        unsafe {
            let mut total: u64 = 0;
            let mut size = std::mem::size_of::<u64>();
            libc::sysctlbyname(
                "hw.memsize\0".as_ptr() as *const i8,
                &mut total as *mut _ as *mut _,
                &mut size,
                std::ptr::null_mut(),
                0,
            );
            (total, total / 4)
        }
    }

    /// Detect GPU devices (nvidia-smi, system_profiler)
    fn detect_gpus() -> Vec<GpuDevice> {
        let mut gpus = Vec::new();
        if let Some(gpu) = Self::detect_nvidia_gpu() {
            gpus.push(gpu);
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(gpu) = Self::detect_metal_gpu() {
                gpus.push(gpu);
            }
        }
        gpus
    }

    fn detect_nvidia_gpu() -> Option<GpuDevice> {
        let output = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,memory.total", "--format=csv,noheader"])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split(", ").collect();
        if parts.len() >= 2 {
            let memory = parts[1]
                .trim()
                .trim_end_matches(" MiB")
                .trim_end_matches(" MB")
                .parse::<u64>()
                .unwrap_or(0)
                * 1024
                * 1024;

            return Some(GpuDevice {
                name: parts[0].trim().to_string(),
                vendor: "NVIDIA".to_string(),
                memory_bytes: memory,
                platform: "CUDA".to_string(),
            });
        }
        None
    }

    #[cfg(target_os = "macos")]
    fn detect_metal_gpu() -> Option<GpuDevice> {
        let output = std::process::Command::new("system_profiler")
            .args(["-xml", "SPDisplaysDataType"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            return Some(GpuDevice {
                name: "Apple GPU".to_string(),
                vendor: "Apple".to_string(),
                memory_bytes: 0,
                platform: "Metal".to_string(),
            });
        }
        None
    }

    pub fn has_gpu(&self) -> bool {
        !self.system_info.gpu_devices.is_empty()
    }

    pub fn best_gpu(&self) -> Option<&GpuDevice> {
        self.system_info
            .gpu_devices
            .iter()
            .max_by_key(|g| g.memory_bytes)
    }
}

/// GPU compute availability info
pub struct GpuAccelerator {
    hardware: Arc<HardwareManager>,
}

impl GpuAccelerator {
    pub fn new(hardware: Arc<HardwareManager>) -> Self {
        Self { hardware }
    }

    pub fn is_available(&self) -> bool {
        self.hardware.has_gpu()
    }

    pub fn gpu_info(&self) -> String {
        if self.hardware.system_info.gpu_devices.is_empty() {
            "No GPU detected".to_string()
        } else {
            self.hardware
                .system_info
                .gpu_devices
                .iter()
                .map(|g| {
                    format!(
                        "{} ({}, {} MB)",
                        g.name,
                        g.platform,
                        g.memory_bytes / (1024 * 1024)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// Result of a file extraction operation
#[derive(Debug)]
pub enum ExtractResult {
    Success(String),
    Failed(String, String),
}

/// Parallel file extractor using rayon
pub struct ParallelExtractor;

impl ParallelExtractor {
    /// Extract files in parallel using rayon thread pool
    pub fn extract_files_parallel(
        files: Vec<(FileEntry, std::path::PathBuf)>,
        reader: &std::sync::Mutex<Box<dyn FilesystemReader + Send>>,
        progress: &crate::progress::ProgressTracker,
        thread_pool: &rayon::ThreadPool,
    ) -> Result<Vec<ExtractResult>, DiskRipperError> {
        let total_files = files.len();
        let completed = AtomicU64::new(0);
        let failed = AtomicU64::new(0);

        let results: Vec<ExtractResult> = thread_pool.install(|| {
            files.into_par_iter()
                .map(|(entry, dest_path)| {
                    if entry.is_dir {
                        let _ = std::fs::create_dir_all(&dest_path);
                        return ExtractResult::Success(entry.path.clone());
                    }

                    if let Some(parent) = dest_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            return ExtractResult::Failed(entry.path.clone(), e.to_string());
                        }
                    }

                    let result = reader.lock().unwrap().read_file(&entry, &dest_path)
                        .map_err(|e| {
                            DiskRipperError::Io(format!("Failed to extract {}: {}", entry.path, e))
                        });

                    let completed_count = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if completed_count % 10 == 0 {
                        progress.add_bytes(entry.size);
                    }

                    match result {
                        Ok(_) => ExtractResult::Success(entry.path.clone()),
                        Err(e) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            ExtractResult::Failed(entry.path.clone(), e.to_string())
                        }
                    }
                })
                .collect()
        });

        let success_count = results.iter().filter(|r| matches!(r, ExtractResult::Success(_))).count();
        let fail_count = results.iter().filter(|r| matches!(r, ExtractResult::Failed(_, _))).count();

        info!(total = total_files, success = success_count, failed = fail_count, "Parallel extraction complete");
        Ok(results)
    }

    /// Compute checksums in parallel for multiple data chunks
    pub fn checksums_parallel(
        chunks: Vec<Vec<u8>>,
        thread_pool: &rayon::ThreadPool,
    ) -> Vec<String> {
        thread_pool.install(|| {
            chunks.into_par_iter()
                .map(|chunk| {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(&chunk);
                    format!("{:x}", hasher.finalize())
                })
                .collect()
        })
    }
}

/// Memory-aware parallel reader
pub struct MemoryAwareReader {
    pub max_memory_bytes: usize,
    pub concurrent_reads: usize,
}

impl MemoryAwareReader {
    pub fn new(max_memory_bytes: usize) -> Self {
        let concurrent_reads = (max_memory_bytes / (8 * 1024 * 1024)).max(1).min(16);
        Self { max_memory_bytes, concurrent_reads }
    }
}

/// Thread-safe progress tracker for parallel operations
pub struct ParallelProgress {
    inner: Arc<std::sync::RwLock<ProgressInfo>>,
}

impl ParallelProgress {
    pub fn new(info: ProgressInfo) -> Self {
        Self { inner: Arc::new(std::sync::RwLock::new(info)) }
    }

    pub fn add_bytes(&self, bytes: u64) {
        if let Ok(mut info) = self.inner.write() {
            info.bytes_processed += bytes;
            info.updated_at = chrono::Utc::now();
        }
    }

    pub fn snapshot(&self) -> Option<ProgressInfo> {
        self.inner.read().ok().map(|info| info.clone())
    }
}
