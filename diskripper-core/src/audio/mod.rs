use std::io;
use std::path::Path;

use crate::error::DiskRipperError;
use crate::filesystem::reader::read_raw_sectors;

/// CD-DA (Digital Audio) sector size: 2352 bytes per sector
pub const CD_DA_SECTOR_SIZE: u32 = 2352;
/// CD-DA sectors per second: 75
pub const CD_DA_SECTORS_PER_SECOND: u32 = 75;
/// CD-DA bytes per sample: 2 (16-bit)
pub const CD_DA_BYTES_PER_SAMPLE: u32 = 2;
/// CD-DA channels: 2 (stereo)
pub const CD_DA_CHANNELS: u32 = 2;
/// CD-DA sample rate: 44100 Hz
pub const CD_DA_SAMPLE_RATE: u32 = 44100;

/// Track information from TOC
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_number: u8,
    pub start_sector: u64,
    pub end_sector: u64,
    pub duration_seconds: f64,
    pub is_audio: bool,
}

impl TrackInfo {
    pub fn sector_count(&self) -> u64 {
        self.end_sector - self.start_sector + 1
    }
}

/// Audio CD reader with jitter correction and TOC support
pub struct AudioCdReader {
    drive_path: String,
    total_sectors: u64,
    jitter_correction: bool,
    toc: Option<Vec<TrackInfo>>,
}

impl AudioCdReader {
    pub fn new(drive_path: String, total_sectors: u64) -> Self {
        Self {
            drive_path,
            total_sectors,
            jitter_correction: true,
            toc: None,
        }
    }

    pub fn with_jitter_correction(mut self, enabled: bool) -> Self {
        self.jitter_correction = enabled;
        self
    }

    /// Read a single audio sector (2352 bytes of raw PCM)
    pub fn read_audio_sector(&self, sector: u64) -> io::Result<Vec<u8>> {
        let data = read_raw_sectors(&self.drive_path, sector, 1, CD_DA_SECTOR_SIZE)?;
        Ok(data)
    }

    /// Read multiple audio sectors
    pub fn read_audio_sectors(&self, start_sector: u64, num_sectors: u32) -> io::Result<Vec<u8>> {
        read_raw_sectors(&self.drive_path, start_sector, num_sectors, CD_DA_SECTOR_SIZE)
    }

    /// Read and verify a sector with jitter correction
    pub fn read_sector_verified(&self, sector: u64, max_retries: u32) -> io::Result<Vec<u8>> {
        if !self.jitter_correction {
            return self.read_audio_sector(sector);
        }

        let mut last_data = Vec::new();
        let mut last_error = None;

        for _attempt in 0..max_retries {
            match self.read_audio_sector(sector) {
                Ok(data) => {
                    if Self::verify_audio_sector(&data) {
                        return Ok(data);
                    }
                    last_data = data;
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        if !last_data.is_empty() {
            Ok(last_data)
        } else {
            Err(last_error.unwrap_or_else(|| io::Error::other("All retries failed")))
        }
    }

    /// Verify that a sector contains valid audio data
    fn verify_audio_sector(data: &[u8]) -> bool {
        if data.len() < CD_DA_SECTOR_SIZE as usize {
            return false;
        }

        let first = data[0];
        let mut all_same = true;
        let mut non_zero_count = 0;

        for &byte in data.iter().take(100) {
            if byte != first {
                all_same = false;
            }
            if byte != 0 {
                non_zero_count += 1;
            }
        }

        !all_same && non_zero_count > 10
    }

    /// Parse the Table of Contents from the drive
    /// Returns track information for all tracks on the disc
    pub fn parse_toc(&mut self) -> io::Result<Vec<TrackInfo>> {
        // Try to read TOC using platform-specific methods
        #[cfg(target_os = "windows")]
        let toc = self.parse_toc_windows()?;
        #[cfg(target_os = "linux")]
        let toc = self.parse_toc_linux()?;
        #[cfg(target_os = "macos")]
        let toc = self.parse_toc_macos()?;
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let toc = self.parse_toc_fallback();

        self.toc = Some(toc.clone());
        Ok(toc)
    }

    /// Get cached TOC or parse if not available
    pub fn get_toc(&mut self) -> io::Result<Vec<TrackInfo>> {
        if let Some(toc) = &self.toc {
            Ok(toc.clone())
        } else {
            self.parse_toc()
        }
    }

    /// Parse TOC on Windows using WMI
    #[cfg(target_os = "windows")]
    fn parse_toc_windows(&self) -> io::Result<Vec<TrackInfo>> {
        use std::process::Command;

        let output = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Get-WmiObject Win32_CDROMDrive | Where-Object {{$_.ID -eq '{}'}} | Select-Object NumberOfTracks, Tracks | ConvertTo-Json",
                    self.drive_path.trim_end_matches('\\')
                ),
            ])
            .output()?;

        if output.status.success() {
            let json = String::from_utf8_lossy(&output.stdout);
            self.parse_windows_toc_json(&json)
        } else {
            Ok(self.parse_toc_fallback())
        }
    }

    #[cfg(target_os = "windows")]
    fn parse_windows_toc_json(&self, json: &str) -> io::Result<Vec<TrackInfo>> {
        // Parse Windows WMI JSON output for track info
        // This is a simplified parser - real implementation would use serde_json
        let mut tracks = Vec::new();
        
        // Extract track count from JSON
        if let Some(tracks_idx) = json.find("\"Tracks\"") {
            if let Some(colon_idx) = json[tracks_idx..].find(':') {
                let start = tracks_idx + colon_idx + 1;
                if let Some(end) = json[start..].find(']') {
                    let tracks_str = &json[start..start + end];
                    // Parse track sectors
                    let mut current_sector = 0u64;
                    for (i, track_str) in tracks_str.split(',').enumerate() {
                        if let Ok(sector) = track_str.trim().parse::<u64>() {
                            let start = current_sector;
                            let end = sector;
                            tracks.push(TrackInfo {
                                track_number: (i + 1) as u8,
                                start_sector: start,
                                end_sector: end,
                                duration_seconds: (end - start + 1) as f64 / CD_DA_SECTORS_PER_SECOND as f64,
                                is_audio: true,
                            });
                            current_sector = end + 1;
                        }
                    }
                }
            }
        }

        if tracks.is_empty() {
            Ok(self.parse_toc_fallback())
        } else {
            Ok(tracks)
        }
    }

    /// Parse TOC on Linux using ioctl
    #[cfg(target_os = "linux")]
    fn parse_toc_linux(&self) -> io::Result<Vec<TrackInfo>> {
        use std::fs::File;
        use std::os::unix::io::AsRawFd;
        use libc::{c_ulong, ioctl, CDROMREADTOCHDR, CDROMREADTOCENTRY, CDROM_LBA, CDROM_MSF, CDROM_DATA_TRACK, CDROM_LEADOUT};

        let file = File::open(&self.drive_path)?;
        let fd = file.as_raw_fd();

        // Read TOC header
        let mut toc_hdr = libc::cdrom_tochdr {
            cdth_trk0: 0,
            cdth_trk1: 0,
        };

        let result = unsafe {
            ioctl(fd, CDROMREADTOCHDR as c_ulong, &mut toc_hdr)
        };

        if result < 0 {
            return Ok(self.parse_toc_fallback());
        }

        let mut tracks = Vec::new();
        let first_track = toc_hdr.cdth_trk0;
        let last_track = toc_hdr.cdth_trk1;

        // Read each track entry
        let mut current_sector = 0u64;
        for track_num in first_track..=last_track {
            let mut entry = libc::cdrom_tocentry {
                cdte_track: track_num,
                cdte_adr: 0,
                cdte_ctrl: 0,
                cdte_format: CDROM_LBA as u8,
                cdte_addr: libc::cdrom_addr { cdmsf_frame: 0, cdmsf_sec: 0, cdmsf_min: 0 },
                cdte_datamode: 0,
            };

            let result = unsafe {
                ioctl(fd, CDROMREADTOCENTRY as c_ulong, &mut entry)
            };

            if result < 0 {
                continue;
            }

            let start_sector = entry.cdte_addr.cdmsf_frame as u64;
            let is_audio = (entry.cdte_ctrl & CDROM_DATA_TRACK as u8) == 0;

            tracks.push(TrackInfo {
                track_number: track_num,
                start_sector: current_sector,
                end_sector: start_sector,
                duration_seconds: (start_sector - current_sector) as f64 / CD_DA_SECTORS_PER_SECOND as f64,
                is_audio,
            });

            current_sector = start_sector;
        }

        // Add lead-out track
        let mut leadout = libc::cdrom_tocentry {
            cdte_track: CDROM_LEADOUT as u8,
            cdte_adr: 0,
            cdte_ctrl: 0,
            cdte_format: CDROM_LBA as u8,
            cdte_addr: libc::cdrom_addr { cdmsf_frame: 0, cdmsf_sec: 0, cdmsf_min: 0 },
            cdte_datamode: 0,
        };

        let result = unsafe {
            ioctl(fd, CDROMREADTOCENTRY as c_ulong, &mut leadout)
        };

        if result >= 0 {
            let leadout_sector = leadout.cdte_addr.cdmsf_frame as u64;
            tracks.push(TrackInfo {
                track_number: 0xAA, // Lead-out
                start_sector: current_sector,
                end_sector: leadout_sector,
                duration_seconds: (leadout_sector - current_sector) as f64 / CD_DA_SECTORS_PER_SECOND as f64,
                is_audio: true,
            });
        }

        if tracks.is_empty() {
            Ok(self.parse_toc_fallback())
        } else {
            Ok(tracks)
        }
    }

    /// Parse TOC on macOS using IOKit
    #[cfg(target_os = "macos")]
    fn parse_toc_macos(&self) -> io::Result<Vec<TrackInfo>> {
        use std::process::Command;

        let output = Command::new("diskutil")
            .args(["info", &self.drive_path])
            .output()?;

        if output.status.success() {
            let info = String::from_utf8_lossy(&output.stdout);
            self.parse_macos_toc(&info)
        } else {
            Ok(self.parse_toc_fallback())
        }
    }

    #[cfg(target_os = "macos")]
    fn parse_macos_toc(&self, info: &str) -> io::Result<Vec<TrackInfo>> {
        // Parse diskutil output for track info
        let mut tracks = Vec::new();
        let mut in_tracks = false;
        let mut current_sector = 0u64;

        for line in info.lines() {
            if line.contains("Tracks:") {
                in_tracks = true;
                continue;
            }
            if in_tracks {
                if line.trim().is_empty() {
                    break;
                }
                // Parse track line: "Track 1: 0 -> 12345"
                if line.contains("Track") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        if let (Ok(track_num), Ok(end_sector)) = (
                            parts[1].trim_end_matches(':').parse::<u8>(),
                            parts[4].parse::<u64>(),
                        ) {
                            tracks.push(TrackInfo {
                                track_number: track_num,
                                start_sector: current_sector,
                                end_sector,
                                duration_seconds: (end_sector - current_sector) as f64 / CD_DA_SECTORS_PER_SECOND as f64,
                                is_audio: true,
                            });
                            current_sector = end_sector + 1;
                        }
                    }
                }
            }
        }

        if tracks.is_empty() {
            Ok(self.parse_toc_fallback())
        } else {
            Ok(tracks)
        }
    }

    /// Fallback TOC parser - creates a single track for the whole disc
    fn parse_toc_fallback(&self) -> Vec<TrackInfo> {
        vec![TrackInfo {
            track_number: 1,
            start_sector: 0,
            end_sector: self.total_sectors - 1,
            duration_seconds: self.total_sectors as f64 / CD_DA_SECTORS_PER_SECOND as f64,
            is_audio: true,
        }]
    }

    /// Calculate audio duration from sector count
    pub fn sectors_to_duration(sectors: u64) -> (u32, u32, u32) {
        let total_seconds = sectors / CD_DA_SECTORS_PER_SECOND as u64;
        let hours = (total_seconds / 3600) as u32;
        let minutes = ((total_seconds % 3600) / 60) as u32;
        let seconds = (total_seconds % 60) as u32;
        (hours, minutes, seconds)
    }

    /// Convert sector count to byte offset for WAV file
    pub fn sectors_to_bytes(sectors: u64) -> u64 {
        sectors * CD_DA_SECTOR_SIZE as u64
    }

    /// Get total sectors
    pub fn total_sectors(&self) -> u64 {
        self.total_sectors
    }

    /// Get total duration as (hours, minutes, seconds)
    pub fn total_duration(&self) -> (u32, u32, u32) {
        Self::sectors_to_duration(self.total_sectors)
    }
}

/// WAV file header for CD-DA audio
pub struct WavHeader;

impl WavHeader {
    /// Generate a WAV header for CD-DA audio
    pub fn new_cd_da(data_size: u32) -> Vec<u8> {
        let mut header = Vec::with_capacity(44);
        let byte_rate = CD_DA_SAMPLE_RATE * CD_DA_CHANNELS as u32 * CD_DA_BYTES_PER_SAMPLE;
        let block_align = CD_DA_CHANNELS as u16 * CD_DA_BYTES_PER_SAMPLE as u16;

        // RIFF chunk
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&(36 + data_size).to_le_bytes());
        header.extend_from_slice(b"WAVE");

        // fmt sub-chunk
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16u32.to_le_bytes());
        header.extend_from_slice(&1u16.to_le_bytes());
        header.extend_from_slice(&(CD_DA_CHANNELS as u16).to_le_bytes());
        header.extend_from_slice(&CD_DA_SAMPLE_RATE.to_le_bytes());
        header.extend_from_slice(&byte_rate.to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&16u16.to_le_bytes());

        // data sub-chunk
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_size.to_le_bytes());

        header
    }
}

/// Extract audio track to WAV file
pub fn extract_audio_track(
    drive_path: &Path,
    start_sector: u64,
    num_sectors: u64,
    output_path: &Path,
    jitter_correction: bool,
) -> Result<(), DiskRipperError> {
    use std::fs::File;
    use std::io::Write;

    let reader = AudioCdReader::new(
        drive_path.to_string_lossy().to_string(),
        num_sectors,
    ).with_jitter_correction(jitter_correction);

    let data_size = (num_sectors as u32) * CD_DA_SECTOR_SIZE;
    let header = WavHeader::new_cd_da(data_size);

    let mut file = File::create(output_path)?;
    file.write_all(&header)?;

    let batch_size = 100u32;
    let mut current_sector = start_sector;
    let mut remaining = num_sectors;

    while remaining > 0 {
        let to_read = if remaining > batch_size as u64 {
            batch_size
        } else {
            remaining as u32
        };

        let data = reader.read_audio_sectors(current_sector, to_read)?;
        file.write_all(&data)?;

        current_sector += to_read as u64;
        remaining -= to_read as u64;
    }

    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sectors_to_duration() {
        let (h, m, s) = AudioCdReader::sectors_to_duration(75 * 60 * 3);
        assert_eq!(h, 0);
        assert_eq!(m, 3);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_sectors_to_bytes() {
        assert_eq!(AudioCdReader::sectors_to_bytes(1), 2352);
        assert_eq!(AudioCdReader::sectors_to_bytes(75), 75 * 2352);
    }

    #[test]
    fn test_wav_header() {
        let header = WavHeader::new_cd_da(2352);
        assert_eq!(header.len(), 44);
        assert_eq!(&header[0..4], b"RIFF");
        assert_eq!(&header[8..12], b"WAVE");
        assert_eq!(&header[12..16], b"fmt ");
        assert_eq!(&header[36..40], b"data");
    }

    #[test]
    fn test_verify_audio_sector_valid() {
        let mut data = vec![0u8; 2352];
        for i in 0..100 {
            data[i] = (i % 256) as u8;
        }
        assert!(AudioCdReader::verify_audio_sector(&data));
    }

    #[test]
    fn test_verify_audio_sector_all_zeros() {
        let data = vec![0u8; 2352];
        assert!(!AudioCdReader::verify_audio_sector(&data));
    }

    #[test]
    fn test_verify_audio_sector_all_same() {
        let data = vec![0xFFu8; 2352];
        assert!(!AudioCdReader::verify_audio_sector(&data));
    }

    #[test]
    fn test_verify_audio_sector_too_small() {
        let data = vec![0u8; 100];
        assert!(!AudioCdReader::verify_audio_sector(&data));
    }

    #[test]
    fn test_track_info_sector_count() {
        let track = TrackInfo {
            track_number: 1,
            start_sector: 0,
            end_sector: 100,
            duration_seconds: 100.0 / 75.0,
            is_audio: true,
        };
        assert_eq!(track.sector_count(), 101);
    }

    #[test]
    fn test_parse_toc_fallback() {
        let reader = AudioCdReader::new("D:\\".to_string(), 337500);
        let toc = reader.parse_toc_fallback();
        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].track_number, 1);
        assert_eq!(toc[0].start_sector, 0);
    }
}
