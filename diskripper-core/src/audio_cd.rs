//! Audio CD ripping to WAV with cue sheet generation.

use std::io::{Write, Seek, SeekFrom};
use std::path::Path;

use tracing::{info, warn};

use crate::error::DiskRipperError;
use crate::filesystem::native_win::NativeDriveHandle;
use crate::progress::ProgressTracker;
use crate::types::*;

/// Audio format for ripping
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioFormat {
    Wav,
    Flac,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Wav => write!(f, "WAV"),
            AudioFormat::Flac => write!(f, "FLAC"),
        }
    }
}

/// Track information for audio CD
#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub track_number: u8,
    pub start_sector: u64,
    pub end_sector: u64,
    pub sector_count: u64,
    pub duration_seconds: f64,
    pub is_audio: bool,
    pub pre_emphasis: bool,
    pub copy_permitted: bool,
    pub channels: u8,
    pub title: Option<String>,
    pub artist: Option<String>,
}

impl AudioTrack {
    /// Get duration as minutes:seconds:frames string
    pub fn duration_msf(&self) -> String {
        let total_frames = self.sector_count * 75;
        let minutes = total_frames / (75 * 60);
        let seconds = (total_frames / 75) % 60;
        let frames = total_frames % 75;
        format!("{:02}:{:02}:{:02}", minutes, seconds, frames)
    }
}

/// Audio CD ripper
pub struct AudioCdRipper;

impl AudioCdRipper {
    /// Get track layout from TOC
    pub fn get_tracks(handle: &NativeDriveHandle) -> Result<Vec<AudioTrack>, DiskRipperError> {
        let toc_tracks = handle.get_toc().map_err(|e| DiskRipperError::ReadError(e.to_string()))?;
        
        let mut tracks = Vec::new();
        for (i, toc_track) in toc_tracks.iter().enumerate() {
            let start_sector = toc_track.start_lba as u64;
            
            let end_sector = if i + 1 < toc_tracks.len() {
                toc_tracks[i + 1].start_lba as u64
            } else {
                match handle.get_disc_size() {
                    Ok(size) => size / 2352,
                    Err(_) => start_sector + 10000,
                }
            };
            
            let sector_count = end_sector.saturating_sub(start_sector);
            let is_audio = (toc_track.control & 0x04) == 0;
            
            tracks.push(AudioTrack {
                track_number: toc_track.track_number,
                start_sector,
                end_sector,
                sector_count,
                duration_seconds: sector_count as f64 / 75.0,
                is_audio,
                pre_emphasis: (toc_track.control & 0x01) != 0,
                copy_permitted: (toc_track.control & 0x02) != 0,
                channels: 2,
                title: None,
                artist: None,
            });
        }
        
        info!(track_count = tracks.len(), "Found {} tracks", tracks.len());
        Ok(tracks)
    }
    
    /// Rip a single track to WAV
    pub fn rip_track_wav(
        handle: &NativeDriveHandle,
        track: &AudioTrack,
        output_path: &Path,
        progress: &ProgressTracker,
    ) -> Result<(), DiskRipperError> {
        info!(
            track = track.track_number,
            start = track.start_sector,
            sectors = track.sector_count,
            "Ripping track to WAV"
        );
        
        let mut file = std::fs::File::create(output_path)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        // Write placeholder WAV header
        write_wav_header(&mut file, 0, 44100, 16, 2)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        // Read and write audio data in chunks
        let chunk_size = 100u32;
        let mut current_sector = track.start_sector;
        let mut total_bytes_written: u32 = 0;
        
        while current_sector < track.end_sector {
            let sectors_to_read = std::cmp::min(chunk_size, (track.end_sector - current_sector) as u32);
            
            match handle.read_cdda_sectors(current_sector, sectors_to_read) {
                Ok(data) => {
                    file.write_all(&data)
                        .map_err(|e| DiskRipperError::Io(e.to_string()))?;
                    total_bytes_written += data.len() as u32;
                    current_sector += sectors_to_read as u64;
                    progress.add_bytes(data.len() as u64);
                }
                Err(e) => {
                    warn!(sector = current_sector, error = %e, "Read error, filling with zeros");
                    let zeros = vec![0u8; (sectors_to_read as usize) * 2352];
                    file.write_all(&zeros)
                        .map_err(|e| DiskRipperError::Io(e.to_string()))?;
                    total_bytes_written += zeros.len() as u32;
                    current_sector += sectors_to_read as u64;
                }
            }
        }
        
        // Update WAV header with actual data size
        file.seek(SeekFrom::Start(0))
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        write_wav_header(&mut file, total_bytes_written, 44100, 16, 2)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        info!(track = track.track_number, bytes = total_bytes_written, "Track rip complete");
        Ok(())
    }
    
    /// Rip all audio tracks to WAV files
    pub fn rip_all_tracks_wav(
        handle: &NativeDriveHandle,
        tracks: &[AudioTrack],
        output_dir: &Path,
        progress: &ProgressTracker,
    ) -> Result<Vec<std::path::PathBuf>, DiskRipperError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        let mut output_paths = Vec::new();
        
        for track in tracks {
            if !track.is_audio {
                continue;
            }
            
            let filename = format!("{:02}.wav", track.track_number);
            let output_path = output_dir.join(&filename);
            
            Self::rip_track_wav(handle, track, &output_path, progress)?;
            output_paths.push(output_path);
        }
        
        Ok(output_paths)
    }
    
    /// Generate cue sheet from track list
    pub fn generate_cue_sheet(
        tracks: &[AudioTrack],
        disc_title: Option<&str>,
        disc_artist: Option<&str>,
        output_path: &Path,
    ) -> Result<(), DiskRipperError> {
        let mut cue = String::new();
        
        if let Some(artist) = disc_artist {
            cue.push_str(&format!("PERFORMER \"{}\"\n", escape_cue_string(artist)));
        }
        if let Some(title) = disc_title {
            cue.push_str(&format!("TITLE \"{}\"\n", escape_cue_string(title)));
        }
        
        for track in tracks.iter() {
            if !track.is_audio {
                continue;
            }
            
            let filename = format!("{:02}.wav", track.track_number);
            cue.push_str(&format!("FILE \"{}\" WAV\n", filename));
            cue.push_str(&format!("  TRACK {:02} AUDIO\n", track.track_number));
            
            // Pre-gap: 2 seconds = 150 sectors
            let pregap_sectors = 150u64;
            let pregap_frames = (pregap_sectors % 75) as u64;
            let pregap_seconds = ((pregap_sectors / 75) % 60) as u64;
            let pregap_minutes = (pregap_sectors / (75 * 60)) as u64;
            
            cue.push_str(&format!(
                "    INDEX 01 {:02}:{:02}:{:02}\n",
                pregap_minutes, pregap_seconds, pregap_frames
            ));
            
            if let Some(title) = &track.title {
                cue.push_str(&format!("    TITLE \"{}\"\n", escape_cue_string(title)));
            }
            if let Some(artist) = &track.artist {
                cue.push_str(&format!("    PERFORMER \"{}\"\n", escape_cue_string(artist)));
            }
            
            let mut flags = Vec::new();
            if track.pre_emphasis {
                flags.push("PRE");
            }
            if track.copy_permitted {
                flags.push("DCP");
            }
            if !flags.is_empty() {
                cue.push_str(&format!("    FLAGS {}\n", flags.join(" ")));
            }
        }
        
        std::fs::write(output_path, cue)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
        
        info!(path = %output_path.display(), "Generated cue sheet");
        Ok(())
    }
    
    /// Rip audio CD with cue sheet
    pub fn rip_audio_cd(
        handle: &NativeDriveHandle,
        output_dir: &Path,
        disc_title: Option<&str>,
        disc_artist: Option<&str>,
        progress: &ProgressTracker,
    ) -> Result<(), DiskRipperError> {
        let tracks = Self::get_tracks(handle)?;
        Self::rip_all_tracks_wav(handle, &tracks, output_dir, progress)?;
        
        let cue_path = output_dir.join("album.cue");
        Self::generate_cue_sheet(&tracks, disc_title, disc_artist, &cue_path)?;
        
        Ok(())
    }
}

/// Write a standard 44.1kHz/16-bit stereo WAV header
fn write_wav_header(
    file: &mut std::fs::File,
    data_size: u32,
    sample_rate: u32,
    bits_per_sample: u16,
    channels: u16,
) -> Result<(), std::io::Error> {
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let file_size = data_size + 36;
    
    let mut header = Vec::new();
    
    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&file_size.to_le_bytes());
    header.extend_from_slice(b"WAVE");
    
    // fmt chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes()); // PCM
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&block_align.to_le_bytes());
    header.extend_from_slice(&bits_per_sample.to_le_bytes());
    
    // data chunk
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_size.to_le_bytes());
    
    file.write_all(&header)?;
    Ok(())
}

/// Escape special characters in cue sheet strings
fn escape_cue_string(s: &str) -> String {
    s.replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
