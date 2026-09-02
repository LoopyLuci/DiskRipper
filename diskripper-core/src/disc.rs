use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscInfo {
    pub disc_type: DiscType,
    pub label: Option<String>,
    pub total_size: u64,
    pub used_size: u64,
    pub free_size: u64,
    pub file_system: FileSystem,
    pub sessions: u32,
    pub tracks: u32,
    pub manufacturer: Option<String>,
    pub write_speed: Option<u32>,
    pub inserted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscType {
    CdRom,
    CdR,
    CdRw,
    DvdRom,
    DvdR,
    DvdRw,
    DvdRam,
    DvdPlusR,
    DvdPlusRw,
    DvdPlusRDL,
    BdRom,
    BdR,
    BdRe,
    BdRDL,
    HdDvd,
    Unknown,
}

impl DiscType {
    pub fn is_writable(&self) -> bool {
        matches!(
            self,
            DiscType::CdR
                | DiscType::CdRw
                | DiscType::DvdR
                | DiscType::DvdRw
                | DiscType::DvdRam
                | DiscType::DvdPlusR
                | DiscType::DvdPlusRw
                | DiscType::DvdPlusRDL
                | DiscType::BdR
                | DiscType::BdRe
                | DiscType::BdRDL
        )
    }

    pub fn is_bluray(&self) -> bool {
        matches!(
            self,
            DiscType::BdRom | DiscType::BdR | DiscType::BdRe | DiscType::BdRDL
        )
    }

    pub fn is_dvd(&self) -> bool {
        matches!(
            self,
            DiscType::DvdRom
                | DiscType::DvdR
                | DiscType::DvdRw
                | DiscType::DvdRam
                | DiscType::DvdPlusR
                | DiscType::DvdPlusRw
                | DiscType::DvdPlusRDL
                | DiscType::HdDvd
        )
    }

    pub fn is_cd(&self) -> bool {
        matches!(self, DiscType::CdRom | DiscType::CdR | DiscType::CdRw)
    }

    pub fn capacity_bytes(&self) -> u64 {
        match self {
            DiscType::CdRom | DiscType::CdR | DiscType::CdRw => 700 * 1024 * 1024,
            DiscType::DvdRom | DiscType::DvdR | DiscType::DvdRw | DiscType::DvdRam => 4_700_000_000,
            DiscType::DvdPlusRDL => 8_500_000_000,
            DiscType::BdRom | DiscType::BdR | DiscType::BdRe => 25_000_000_000,
            DiscType::BdRDL => 50_000_000_000,
            DiscType::HdDvd => 15_000_000_000,
            _ => 0,
        }
    }
}

impl std::fmt::Display for DiscType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscType::CdRom => write!(f, "CD-ROM"),
            DiscType::CdR => write!(f, "CD-R"),
            DiscType::CdRw => write!(f, "CD-RW"),
            DiscType::DvdRom => write!(f, "DVD-ROM"),
            DiscType::DvdR => write!(f, "DVD-R"),
            DiscType::DvdRw => write!(f, "DVD-RW"),
            DiscType::DvdRam => write!(f, "DVD-RAM"),
            DiscType::DvdPlusR => write!(f, "DVD+R"),
            DiscType::DvdPlusRw => write!(f, "DVD+RW"),
            DiscType::DvdPlusRDL => write!(f, "DVD+R DL"),
            DiscType::BdRom => write!(f, "BD-ROM"),
            DiscType::BdR => write!(f, "BD-R"),
            DiscType::BdRe => write!(f, "BD-RE"),
            DiscType::BdRDL => write!(f, "BD-R DL"),
            DiscType::HdDvd => write!(f, "HD DVD"),
            DiscType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileSystem {
    Iso9660,
    Joliet,
    Udf,
    Fat,
    Ntfs,
    HfsPlus,
    Hybrid,
    Unknown,
}

impl std::fmt::Display for FileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileSystem::Iso9660 => write!(f, "ISO 9660"),
            FileSystem::Joliet => write!(f, "Joliet"),
            FileSystem::Udf => write!(f, "UDF"),
            FileSystem::Fat => write!(f, "FAT"),
            FileSystem::Ntfs => write!(f, "NTFS"),
            FileSystem::HfsPlus => write!(f, "HFS+"),
            FileSystem::Hybrid => write!(f, "Hybrid"),
            FileSystem::Unknown => write!(f, "Unknown"),
        }
    }
}

pub trait DiscAnalyzer {
    fn analyze(&self, drive_path: &str) -> Result<DiscInfo, crate::error::DiskRipperError>;
    fn get_disc_size(&self, drive_path: &str) -> Option<u64>;
}

pub struct PlatformDiscAnalyzer;

impl PlatformDiscAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Detect disc type using platform-specific methods
    #[cfg(target_os = "windows")]
    fn detect_type_windows(&self, drive_path: &str) -> DiscType {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

        let raw_path = format!(
            "\\\\.\\{}:",
            drive_path.trim_end_matches('\\').trim_end_matches(':')
        );

        // Try to open the raw device to see if it's accessible
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .custom_flags(0)
            .open(&raw_path)
        {
            Ok(f) => f,
            Err(_) => return DiscType::Unknown,
        };

        // Try to seek to Blu-ray capacity to see if data is there
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::io::BufReader::with_capacity(2048 * 100, file);
        
        // Try reading at BD-ROM offsets to detect disc type
        // BD-ROM: ~25GB, DVD: ~4.7GB, CD: ~700MB
        let test_offsets: [(u64, DiscType); 3] = [
            (25_000_000_000 - 2048, DiscType::BdRom),  // Near end of BD-ROM
            (8_500_000_000 - 2048, DiscType::DvdPlusRDL), // Near end of DVD DL
            (4_700_000_000 - 2048, DiscType::DvdRom),  // Near end of DVD
        ];

        for (offset, disc_type) in test_offsets {
            if file.seek(SeekFrom::Start(offset)).is_ok() {
                let mut buf = [0u8; 2048];
                if file.read_exact(&mut buf).is_ok() {
                    return disc_type;
                }
            }
        }

        // If we can read at CD offsets but not above, it's a CD
        let mut buf = [0u8; 2048];
        if file.seek(SeekFrom::Start(0)).is_ok() && file.read_exact(&mut buf).is_ok() {
            DiscType::CdRom
        } else {
            DiscType::Unknown
        }
    }

    #[allow(dead_code)]
    #[cfg(target_os = "windows")]
    fn parse_windows_media_type(json: &str) -> DiscType {
        // Windows MediaType values - check more specific patterns first
        if json.contains("CD-ROM") {
            DiscType::CdRom
        } else if json.contains("CD-RW") {
            DiscType::CdRw
        } else if json.contains("CD-R") {
            DiscType::CdR
        } else if json.contains("DVD-ROM") {
            DiscType::DvdRom
        } else if json.contains("DVD-RAM") {
            DiscType::DvdRam
        } else if json.contains("DVD-RW") {
            DiscType::DvdRw
        } else if json.contains("DVD+R") {
            DiscType::DvdPlusR
        } else if json.contains("DVD-R") {
            DiscType::DvdR
        } else if json.contains("BD-ROM") {
            DiscType::BdRom
        } else if json.contains("BD-R") {
            DiscType::BdR
        } else {
            DiscType::Unknown
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_type_linux(&self, drive_path: &str) -> DiscType {
        use std::fs;
        use std::path::Path;

        // Check for Blu-ray: /dev/sr* with BD detection
        let path = Path::new(drive_path);
        if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            
            // Check if it's an optical drive
            if name.starts_with("sr") {
                // Try to read disc type from /proc/sys/dev/cdrom/info
                if let Ok(info) = fs::read_to_string("/proc/sys/dev/cdrom/info") {
                    return Self::parse_linux_disc_info(&info, &name);
                }
            }
        }

        DiscType::Unknown
    }

    #[cfg(target_os = "linux")]
    fn parse_linux_disc_info(info: &str, drive_name: &str) -> DiscType {
        // Parse /proc/sys/dev/cdrom/info for drive-specific disc type
        let mut in_drive = false;
        for line in info.lines() {
            if line.contains(&format!("drive name: {}", drive_name)) {
                in_drive = true;
            } else if in_drive {
                if line.contains("Can read CD") {
                    return DiscType::CdRom;
                } else if line.contains("Can read DVD") {
                    return DiscType::DvdRom;
                } else if line.contains("Can read BD") {
                    return DiscType::BdRom;
                }
            }
        }
        DiscType::Unknown
    }

    #[cfg(target_os = "macos")]
    fn detect_type_macos(&self, drive_path: &str) -> DiscType {
        use std::process::Command;

        // Use diskutil to get disc info
        let output = Command::new("diskutil")
            .args(["info", drive_path])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let info = String::from_utf8_lossy(&out.stdout);
                Self::parse_macos_disc_info(&info)
            }
            _ => DiscType::Unknown,
        }
    }

    #[cfg(target_os = "macos")]
    fn parse_macos_disc_info(info: &str) -> DiscType {
        if info.contains("BD-ROM") || info.contains("Blu-ray") {
            DiscType::BdRom
        } else if info.contains("DVD-ROM") {
            DiscType::DvdRom
        } else if info.contains("DVD-R") {
            DiscType::DvdR
        } else if info.contains("DVD+R") {
            DiscType::DvdPlusR
        } else if info.contains("CD-ROM") {
            DiscType::CdRom
        } else if info.contains("CD-R") {
            DiscType::CdR
        } else {
            DiscType::Unknown
        }
    }
}

impl Default for PlatformDiscAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscAnalyzer for PlatformDiscAnalyzer {
    fn analyze(&self, drive_path: &str) -> Result<DiscInfo, crate::error::DiskRipperError> {
        #[cfg(target_os = "windows")]
        let disc_type = self.detect_type_windows(drive_path);
        #[cfg(target_os = "linux")]
        let disc_type = self.detect_type_linux(drive_path);
        #[cfg(target_os = "macos")]
        let disc_type = self.detect_type_macos(drive_path);
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let disc_type = DiscType::Unknown;

        let capacity = disc_type.capacity_bytes();

        Ok(DiscInfo {
            disc_type,
            label: None,
            total_size: capacity,
            used_size: 0,
            free_size: capacity,
            file_system: FileSystem::Unknown,
            sessions: 1,
            tracks: 1,
            manufacturer: None,
            write_speed: None,
            inserted_at: Utc::now(),
        })
    }

    fn get_disc_size(&self, drive_path: &str) -> Option<u64> {
        crate::filesystem::reader::get_disc_size(drive_path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disc_type_display() {
        assert_eq!(DiscType::CdRom.to_string(), "CD-ROM");
        assert_eq!(DiscType::BdRom.to_string(), "BD-ROM");
        assert_eq!(DiscType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_disc_type_capacity() {
        assert_eq!(DiscType::CdRom.capacity_bytes(), 700 * 1024 * 1024);
        assert_eq!(DiscType::DvdRom.capacity_bytes(), 4_700_000_000);
        assert_eq!(DiscType::BdRom.capacity_bytes(), 25_000_000_000);
    }

    #[test]
    fn test_disc_type_writable() {
        assert!(DiscType::CdR.is_writable());
        assert!(DiscType::BdR.is_writable());
        assert!(!DiscType::CdRom.is_writable());
        assert!(!DiscType::BdRom.is_writable());
    }

    #[test]
    fn test_parse_windows_media_type() {
        assert_eq!(
            PlatformDiscAnalyzer::parse_windows_media_type("CD-ROM"),
            DiscType::CdRom
        );
        assert_eq!(
            PlatformDiscAnalyzer::parse_windows_media_type("BD-ROM"),
            DiscType::BdRom
        );
        assert_eq!(
            PlatformDiscAnalyzer::parse_windows_media_type("DVD-RAM"),
            DiscType::DvdRam
        );
    }
}
