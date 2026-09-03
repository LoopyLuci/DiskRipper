//! Native Windows raw disc access using Win32 API via `windows` crate.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::sync::Mutex;

use tracing::{debug, warn};
use windows::Win32::Foundation as Found;
use windows::Win32::Storage::FileSystem as FS;
use windows::Win32::System::IO;

// Win32 constants
const GENERIC_READ: u32 = 0x80000000;
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;
const FILE_BEGIN: u32 = 0;

// IOCTL codes
const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007405C;
const IOCTL_CDROM_RAW_READ: u32 = 0x0002403E;
const IOCTL_CDROM_GET_TOC: u32 = 0x0004D00E;

/// Persistent handle for raw disc access
pub struct NativeDriveHandle {
    handle: Found::HANDLE,
    path: String,
}

impl NativeDriveHandle {
    /// Get the raw handle
    pub fn handle(&self) -> Found::HANDLE {
        self.handle
    }
    
    /// Get the drive path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Open a drive for raw reading (e.g., "D:" or "D:\\" or "\\\\.\\D:")
    pub fn open(drive_path: &str) -> io::Result<Self> {
        let wide_path = Self::to_raw_device_path(drive_path);

        let handle = unsafe {
            FS::CreateFileW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                GENERIC_READ,
                FS::FILE_SHARE_READ | FS::FILE_SHARE_WRITE,
                None,
                FS::OPEN_EXISTING,
                FS::FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        };

        if handle.is_err() {
            return Err(io::Error::other(format!(
                "CreateFileW failed: {:?}",
                handle.err()
            )));
        }

        let handle = handle.unwrap();
        let path = drive_path.to_string();
        debug!(path = %path, "Opened native drive handle");
        Ok(Self { handle, path })
    }

    /// Convert drive path to raw device path
    fn to_raw_device_path(drive_path: &str) -> Vec<u16> {
        let clean = drive_path.trim_end_matches('\\');
        let device_str = if clean.len() == 1 && clean.chars().next().unwrap().is_ascii_alphabetic() {
            format!(r"\\.\{}:", clean.to_uppercase())
        } else if clean.len() == 2 && clean.ends_with(':') {
            format!(r"\\.\{}", clean.to_uppercase())
        } else {
            clean.to_string()
        };

        OsStr::new(&device_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Get total disc size in bytes using IOCTL_DISK_GET_LENGTH_INFO
    pub fn get_disc_size(&self) -> io::Result<u64> {
        let mut info = GET_LENGTH_INFORMATION { length: 0 };
        let mut bytes_returned = 0u32;

        let result = unsafe {
            IO::DeviceIoControl(
                self.handle,
                IOCTL_DISK_GET_LENGTH_INFO,
                None,
                0,
                Some(&mut info as *mut _ as *mut _),
                std::mem::size_of::<GET_LENGTH_INFORMATION>() as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        if result.is_err() {
            return Err(io::Error::other("IOCTL_DISK_GET_LENGTH_INFO failed"));
        }

        Ok(info.length as u64)
    }

    /// Read standard data sectors (2048 bytes each for Mode 1)
    pub fn read_data_sectors(
        &self,
        start_sector: u64,
        num_sectors: u32,
        sector_size: u32,
    ) -> io::Result<Vec<u8>> {
        let total_bytes = (num_sectors as usize) * (sector_size as usize);
        let mut buffer = vec![0u8; total_bytes];

        // Seek to position
        let offset = start_sector * sector_size as u64;
        let mut new_pos = 0i64;
        let seek_result = unsafe {
            FS::SetFilePointerEx(
                self.handle,
                offset as i64,
                Some(&mut new_pos),
                FS::FILE_BEGIN,
            )
        };

        if seek_result.is_err() {
            return Err(io::Error::other("SetFilePointerEx failed"));
        }

        // Read data
        let mut bytes_read = 0u32;
        let read_result = unsafe {
            FS::ReadFile(
                self.handle,
                Some(&mut buffer[..]),
                Some(&mut bytes_read),
                None,
            )
        };

        if read_result.is_err() {
            return Err(io::Error::other("ReadFile failed"));
        }

        buffer.truncate(bytes_read as usize);
        Ok(buffer)
    }

    /// Read CDDA audio sectors (2352 bytes each) using IOCTL_CDROM_RAW_READ
    pub fn read_cdda_sectors(&self, start_sector: u64, num_sectors: u32) -> io::Result<Vec<u8>> {
        let mut read_info = RAW_READ_INFO {
            disk_offset: (start_sector * 2352) as i64,
            sector_count: num_sectors,
            track_mode: TRACK_MODE_CDDA,
        };

        let total_bytes = (num_sectors as usize) * 2352;
        let mut buffer = vec![0u8; total_bytes];
        let mut bytes_returned = 0u32;

        let result = unsafe {
            IO::DeviceIoControl(
                self.handle,
                IOCTL_CDROM_RAW_READ,
                Some(&mut read_info as *mut _ as *mut _),
                std::mem::size_of::<RAW_READ_INFO>() as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                total_bytes as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        if result.is_err() {
            return Err(io::Error::other("IOCTL_CDROM_RAW_READ failed"));
        }

        buffer.truncate(bytes_returned as usize);
        Ok(buffer)
    }

    /// Read Table of Contents for track layout
    pub fn get_toc(&self) -> io::Result<Vec<TocTrack>> {
        let mut toc: CDROM_TOC = unsafe { std::mem::zeroed() };
        let mut bytes_returned = 0u32;

        let result = unsafe {
            IO::DeviceIoControl(
                self.handle,
                IOCTL_CDROM_GET_TOC,
                None,
                0,
                Some(&mut toc as *mut _ as *mut _),
                std::mem::size_of::<CDROM_TOC>() as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        if result.is_err() {
            return Err(io::Error::other("IOCTL_CDROM_GET_TOC failed"));
        }

        let mut tracks = Vec::new();
        let first = toc.first_track;
        let last = toc.last_track;

        for i in first..=last {
            let idx = (i - 1) as usize;
            if idx < 100 {
                let track = &toc.track_data[idx];
                // MSF format: address[0]=frame, [1]=second, [2]=minute
                let lba = ((track.address[2] as u32) * 60 * 75)
                    + ((track.address[1] as u32) * 75)
                    + (track.address[0] as u32)
                    - 150; // Convert MSF to LBA

                tracks.push(TocTrack {
                    track_number: i,
                    control: track.control,
                    start_lba: lba,
                });
            }
        }

        Ok(tracks)
    }
}

impl Drop for NativeDriveHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = Found::CloseHandle(self.handle);
        }
    }
}

/// TOC track entry
#[derive(Debug, Clone)]
pub struct TocTrack {
    pub track_number: u8,
    pub control: u8,
    pub start_lba: u32,
}

/// Disc type detection result
#[derive(Debug, Clone)]
pub struct DiscTypeInfo {
    pub filesystem: DiscFilesystem,
    pub total_size: u64,
    pub sector_count: u64,
    pub tracks: Vec<TocTrack>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscFilesystem {
    Iso9660,
    Joliet,
    Udf,
    Hfs,
    Hybrid,
    Unknown,
}

/// Detect disc type by reading sector 16 (PVD) and checking signatures
pub fn detect_disc_type(handle: &NativeDriveHandle) -> io::Result<DiscTypeInfo> {
    // Try to read sector 16 (ISO 9660 PVD location)
    let pvd_data = handle.read_data_sectors(16, 1, 2048)?;

    let filesystem = if pvd_data.len() >= 0x8010 {
        let sig = &pvd_data[0x8001..0x8006];
        if sig == b"CD001" {
            // Check for Joliet
            if pvd_data.len() >= 0x8835 {
                let joliet_sig = &pvd_data[0x8801..0x8806];
                if joliet_sig == b"CD001" {
                    DiscFilesystem::Joliet
                } else {
                    DiscFilesystem::Iso9660
                }
            } else {
                DiscFilesystem::Iso9660
            }
        } else if sig == b"NSR02" || sig == b"NSR03" {
            DiscFilesystem::Udf
        } else {
            // Check for HFS
            if pvd_data.len() >= 0x438 && &pvd_data[0x400..0x402] == b"HX" {
                DiscFilesystem::Hfs
            } else {
                DiscFilesystem::Unknown
            }
        }
    } else {
        DiscFilesystem::Unknown
    };

    // Get total size
    let total_size = handle.get_disc_size().unwrap_or(0);
    let sector_count = total_size / 2048;

    // Get TOC for track info
    let tracks = handle.get_toc().unwrap_or_default();

    Ok(DiscTypeInfo {
        filesystem,
        total_size,
        sector_count,
        tracks,
    })
}

/// Bad sector recovery configuration
#[derive(Debug, Clone)]
pub struct BadSectorConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub skip_on_failure: bool,
    pub max_consecutive_errors: u32,
}

impl Default for BadSectorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            skip_on_failure: true,
            max_consecutive_errors: 50,
        }
    }
}

/// Read sectors with bad sector recovery
pub fn read_sectors_with_recovery(
    handle: &NativeDriveHandle,
    start_sector: u64,
    num_sectors: u32,
    sector_size: u32,
    config: &BadSectorConfig,
) -> io::Result<Vec<u8>> {
    let mut last_error = None;
    let mut delay = config.initial_delay_ms;

    for attempt in 0..config.max_retries {
        match handle.read_data_sectors(start_sector, num_sectors, sector_size) {
            Ok(data) => return Ok(data),
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries - 1 {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                    delay = ((delay as f64) * config.backoff_multiplier) as u64;
                    if delay > config.max_delay_ms {
                        delay = config.max_delay_ms;
                    }
                }
            }
        }
    }

    if config.skip_on_failure {
        warn!(
            sector = start_sector,
            sectors = num_sectors,
            error = %last_error.as_ref().unwrap(),
            "Skipping bad sectors after {} retries",
            config.max_retries
        );
        Ok(vec![0u8; (num_sectors * sector_size) as usize])
    } else {
        Err(io::Error::other(format!(
            "Failed after {} retries: {}",
            config.max_retries,
            last_error.unwrap()
        )))
    }
}

/// Thread-safe wrapper for native drive handle
pub struct ThreadSafeDriveHandle {
    inner: Mutex<NativeDriveHandle>,
}

impl ThreadSafeDriveHandle {
    pub fn open(drive_path: &str) -> io::Result<Self> {
        let handle = NativeDriveHandle::open(drive_path)?;
        Ok(Self {
            inner: Mutex::new(handle),
        })
    }

    pub fn read_data_sectors(
        &self,
        start_sector: u64,
        num_sectors: u32,
        sector_size: u32,
    ) -> io::Result<Vec<u8>> {
        self.inner
            .lock()
            .unwrap()
            .read_data_sectors(start_sector, num_sectors, sector_size)
    }

    pub fn read_cdda_sectors(&self, start_sector: u64, num_sectors: u32) -> io::Result<Vec<u8>> {
        self.inner
            .lock()
            .unwrap()
            .read_cdda_sectors(start_sector, num_sectors)
    }

    pub fn get_disc_size(&self) -> io::Result<u64> {
        self.inner.lock().unwrap().get_disc_size()
    }

    pub fn get_toc(&self) -> io::Result<Vec<TocTrack>> {
        self.inner.lock().unwrap().get_toc()
    }
}

// --- Structures ---

#[repr(C)]
struct GET_LENGTH_INFORMATION {
    length: i64,
}

#[repr(C)]
struct RAW_READ_INFO {
    disk_offset: i64,
    sector_count: u32,
    track_mode: u32,
}

#[repr(C)]
struct CDROM_TOC {
    first_track: u8,
    last_track: u8,
    track_data: [CDROM_TOC_TRACK_DATA; 100],
}

#[repr(C)]
struct CDROM_TOC_TRACK_DATA {
    address: [u8; 4],
    control: u8,
    track_number: u8,
}

const TRACK_MODE_CDDA: u32 = 2;
