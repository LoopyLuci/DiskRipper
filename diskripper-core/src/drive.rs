use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub id: String,
    pub path: String,
    pub label: Option<String>,
    pub drive_type: DriveType,
    pub has_disc: bool,
    pub disc_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriveType {
    Cd,
    Dvd,
    BluRay,
    Hdd,
    Unknown,
}

impl std::fmt::Display for DriveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveType::Cd => write!(f, "CD-ROM"),
            DriveType::Dvd => write!(f, "DVD-ROM"),
            DriveType::BluRay => write!(f, "Blu-ray"),
            DriveType::Hdd => write!(f, "HDD"),
            DriveType::Unknown => write!(f, "Unknown"),
        }
    }
}

pub trait DriveScanner {
    fn scan_drives(&self) -> Vec<DriveInfo>;
    fn get_drive(&self, id: &str) -> Option<DriveInfo>;
    fn has_disc(&self, id: &str) -> bool;
}

pub struct PlatformDriveScanner;

impl PlatformDriveScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlatformDriveScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl DriveScanner for PlatformDriveScanner {
    fn scan_drives(&self) -> Vec<DriveInfo> {
        #[cfg(target_os = "windows")]
        {
            windows::scan_drives()
        }
        #[cfg(target_os = "linux")]
        {
            linux::scan_drives()
        }
        #[cfg(target_os = "macos")]
        {
            macos::scan_drives()
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            vec![]
        }
    }

    fn get_drive(&self, id: &str) -> Option<DriveInfo> {
        self.scan_drives().into_iter().find(|d| d.id == id)
    }

    fn has_disc(&self, id: &str) -> bool {
        self.get_drive(id).map(|d| d.has_disc).unwrap_or(false)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    pub fn scan_drives() -> Vec<DriveInfo> {
        use std::process::Command;
        use std::sync::mpsc;
        use std::time::Duration;

        // Spawn the scan in a thread with a timeout so a slow/unresponsive
        // optical drive never blocks the main thread indefinitely.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = || {
                let output = Command::new("powershell")
                    .args(&[
                        "-NoProfile",
                        "-Command",
                        "Get-CimInstance -ClassName Win32_LogicalDisk -OperationTimeoutSec 10 | Select-Object DeviceID, DriveType, VolumeName | ConvertTo-Json",
                    ])
                    .output();

                match output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        parse_drive_json(&stdout)
                    }
                    Err(_) => vec![],
                }
            };
            let _ = tx.send(result());
        });

        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(drives) => drives,
            Err(_) => {
                eprintln!("[drive scanner] timed out after 15s, returning empty list");
                vec![]
            }
        }
    }

    fn parse_drive_json(json: &str) -> Vec<DriveInfo> {
        #[derive(serde::Deserialize)]
        struct RawDrive {
            #[serde(rename = "DeviceID")]
            device_id: String,
            #[serde(rename = "DriveType")]
            drive_type: u32,
            #[serde(rename = "VolumeName")]
            volume_name: Option<String>,
        }

        let drives: Vec<RawDrive> = match serde_json::from_str(json) {
            Ok(d) => d,
            Err(_) => {
                // Single drive comes back as object, not array
                match serde_json::from_str::<RawDrive>(json) {
                    Ok(d) => vec![d],
                    Err(_) => return vec![],
                }
            }
        };

        drives
            .into_iter()
            .filter(|d| d.drive_type == 5) // Only CD-ROM/Blu-ray drives
            .map(|d| {
                // Try to detect drive type from the volume name or use Blu-ray detection
                let drive_type = match d.volume_name.as_deref() {
                    Some(name) if name.contains("BD") || name.contains("BLU") => DriveType::BluRay,
                    Some(name) if name.contains("DVD") => DriveType::Dvd,
                    _ => DriveType::Cd,
                };
                DriveInfo {
                    id: d.device_id.clone(),
                    path: format!("{}\\", d.device_id),
                    label: d.volume_name,
                    drive_type,
                    has_disc: true,
                    disc_present: true,
                }
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::fs;

    pub fn scan_drives() -> Vec<DriveInfo> {
        let mut drives = vec![];

        // Check /dev/sr* (SCSI CD/DVD/Blu-ray)
        if let Ok(entries) = fs::read_dir("/dev") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("sr") {
                    let path = format!("/dev/{}", name);
                    let mount_path = find_mount_path(&path);
                    drives.push(DriveInfo {
                        id: path.clone(),
                        path: mount_path.unwrap_or_else(|| path.clone()),
                        label: None,
                        drive_type: DriveType::Unknown,
                        has_disc: true,
                        disc_present: true,
                    });
                }
            }
        }

        // Check /media and /mnt for mounted optical media
        for base in &["/media", "/mnt"] {
            if let Ok(entries) = fs::read_dir(base) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        drives.push(DriveInfo {
                            id: entry.path().to_string_lossy().to_string(),
                            path: entry.path().to_string_lossy().to_string(),
                            label: entry.file_name().to_string_lossy().to_string().into(),
                            drive_type: DriveType::Unknown,
                            has_disc: true,
                            disc_present: true,
                        });
                    }
                }
            }
        }

        drives
    }

    fn find_mount_path(dev_path: &str) -> Option<String> {
        let mounts = fs::read_to_string("/proc/mounts").ok()?;
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == dev_path {
                return Some(parts[1].to_string());
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::fs;

    pub fn scan_drives() -> Vec<DriveInfo> {
        let mut drives = vec![];

        if let Ok(entries) = fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    drives.push(DriveInfo {
                        id: path.to_string_lossy().to_string(),
                        path: path.to_string_lossy().to_string(),
                        label: entry.file_name().to_string_lossy().to_string().into(),
                        drive_type: DriveType::Unknown,
                        has_disc: true,
                        disc_present: true,
                    });
                }
            }
        }

        drives
    }
}
