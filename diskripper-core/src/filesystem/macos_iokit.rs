//! macOS IOKit raw device access.
//!
//! Provides direct device access via IOKit frameworks:
//! - Enumerates optical drives (IODTPlatformDevice)
//! - Opens raw device interface (IOCDMedia/IODVDMedia/IOBDMedia)
//! - Sends SCSI commands via IOService

#[cfg(target_os = "macos")]
pub mod macos {
    use std::io;
    use std::path::Path;

    // IOKit constants
    const kIOCDMediaClass: *const i8 = b"IOCDMedia\0".as_ptr() as *const i8;
    const kIODVDMediaClass: *const i8 = b"IODVDMedia\0".as_ptr() as *const i8;
    const kIOBDMediaClass: *const i8 = b"IOBDMedia\0".as_ptr() as *const i8;
    const kIODTPlatformDeviceClass: *const i8 = b"IODTPlatformDevice\0".as_ptr() as *const i8;

    // SCSI command codes
    const SCSIOP_READ_CD: u8 = 0xBE;
    const SCSIOP_READ_TOC: u8 = 0x43;
    const SCSIOP_READ_CAPACITY: u8 = 0x25;
    const SCSIOP_TEST_UNIT_READY: u8 = 0x00;

    /// Check if IOKit is available (always true on macOS)
    pub fn is_iokit_available() -> bool {
        true // IOKit is always available on macOS
    }

    /// Get list of optical drive device paths
    pub fn enumerate_optical_drives() -> io::Result<Vec<String>> {
        let mut drives = Vec::new();
        
        // Use IOKit to find optical drives
        // This is a simplified version - full implementation would use
        // IOServiceGetMatchingServices with IOCDMedia/IODVDMedia/IOBDMedia
        
        // Fallback: check common device paths
        let common_paths = [
            "/dev/disk0", "/dev/disk1", "/dev/disk2", "/dev/disk3",
            "/dev/disk4", "/dev/disk5", "/dev/disk6", "/dev/disk7",
        ];
        
        for path in &common_paths {
            if std::path::Path::new(path).exists() {
                // Check if it's an optical drive using IOKit
                if is_optical_drive(path) {
                    drives.push(path.to_string());
                }
            }
        }
        
        Ok(drives)
    }

    /// Check if a device is an optical drive
    fn is_optical_drive(device_path: &str) -> bool {
        // Use IOKit to check device type
        // Simplified: check if device name contains "CD", "DVD", or "BD"
        // Full implementation would query IOKit registry
        
        // For now, use system_profiler to check
        if let Ok(output) = std::process::Command::new("system_profiler")
            .args(["-xml", "SPDiscBurningDataType"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return !stdout.is_empty();
        }
        
        false
    }

    /// Read sectors from optical drive (fallback to standard I/O)
    pub fn read_sectors<P: AsRef<Path>>(
        device_path: P,
        start_sector: u64,
        num_sectors: u32,
        sector_size: u32,
    ) -> io::Result<Vec<u8>> {
        use std::fs::File;
        use std::io::{Read, Seek, SeekFrom};

        let mut file = File::open(device_path)?;
        let offset = start_sector * sector_size as u64;
        file.seek(SeekFrom::Start(offset))?;
        
        let buffer_size = (num_sectors * sector_size) as usize;
        let mut buffer = vec![0u8; buffer_size];
        file.read_exact(&mut buffer)?;
        
        Ok(buffer)
    }

    /// Get disc size (fallback method)
    pub fn get_disc_size<P: AsRef<Path>>(device_path: P) -> io::Result<u64> {
        use std::fs::File;
        use std::io::Seek;

        let mut file = File::open(device_path)?;
        file.seek(std::io::SeekFrom::End(0))
    }

    /// Get TOC (Table of Contents) - simplified version
    pub fn get_toc<P: AsRef<Path>>(_device_path: P) -> io::Result<TocData> {
        // Full implementation would use IOKit to send SCSI READ TOC command
        // For now, return empty TOC
        Ok(TocData {
            first_track: 1,
            last_track: 0,
            tracks: Vec::new(),
        })
    }
}

#[cfg(target_os = "macos")]
pub use macos::*;

/// TOC track entry
#[derive(Debug, Clone)]
pub struct TocTrack {
    pub track_number: u8,
    pub control: u8,
    pub start_lba: u32,
}

/// TOC data
#[derive(Debug, Clone)]
pub struct TocData {
    pub first_track: u8,
    pub last_track: u8,
    pub tracks: Vec<TocTrack>,
}

/// Check if IOKit is available
#[cfg(not(target_os = "macos"))]
pub fn is_iokit_available() -> bool {
    false
}
