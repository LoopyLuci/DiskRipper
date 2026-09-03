//! Multisession and mixed-mode CD support.
//!
//! Many CDs use multiple sessions (especially game CDs, enhanced CDs).
//! Mixed-mode CDs contain both audio tracks and data tracks.
//!
//! This module provides:
//! - Session detection and enumeration
//! - Track type detection (audio vs data)
//! - Multisession reading
//! - Mixed-mode CD support

use std::collections::HashMap;

use tracing::{info, warn};

use crate::error::DiskRipperError;

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_number: u8,
    pub start_lba: u32,
    pub end_lba: u32,
    pub track_count: u8,
    pub tracks: Vec<TrackInfo>,
}

/// Track information
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_number: u8,
    pub start_lba: u32,
    pub end_lba: u32,
    pub sector_count: u64,
    pub track_type: TrackType,
    pub pre_emphasis: bool,
    pub copy_permitted: bool,
    pub channels: u8,
    pub isrc: Option<String>,
    pub title: Option<String>,
}

/// Track type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    Audio,
    Data,
    Mode1,
    Mode2,
    Unknown,
}

impl std::fmt::Display for TrackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackType::Audio => write!(f, "Audio"),
            TrackType::Data => write!(f, "Data"),
            TrackType::Mode1 => write!(f, "Mode 1"),
            TrackType::Mode2 => write!(f, "Mode 2"),
            TrackType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// CD table of contents
#[derive(Debug, Clone)]
pub struct CdToc {
    pub sessions: Vec<SessionInfo>,
    pub total_sessions: u8,
    pub total_tracks: u8,
    pub lead_out_lba: u32,
}

/// Multisession CD reader
pub struct MultisessionReader;

impl MultisessionReader {
    /// Read TOC from CD using SCSI READ TOC command
    ///
    /// Returns all sessions and their tracks.
    pub fn read_toc(drive_path: &str) -> Result<CdToc, DiskRipperError> {
        // Placeholder: In production, this would:
        // 1. Send SCSI READ TOC/PMA/ATIP command (0x43)
        // 2. Parse the returned TOC data
        // 3. Build session and track information

        warn!("Multisession TOC reading not yet implemented for {}", drive_path);
        Ok(CdToc {
            sessions: Vec::new(),
            total_sessions: 0,
            total_tracks: 0,
            lead_out_lba: 0,
        })
    }

    /// Detect if a CD is mixed-mode
    ///
    /// Mixed-mode CDs have both audio and data tracks.
    /// The first track is typically data, followed by audio tracks.
    pub fn is_mixed_mode(toc: &CdToc) -> bool {
        let mut has_audio = false;
        let mut has_data = false;

        for session in &toc.sessions {
            for track in &session.tracks {
                match track.track_type {
                    TrackType::Audio => has_audio = true,
                    TrackType::Data | TrackType::Mode1 | TrackType::Mode2 => has_data = true,
                    _ => {}
                }
            }
        }

        has_audio && has_data
    }

    /// Get all audio tracks from TOC
    pub fn get_audio_tracks(toc: &CdToc) -> Vec<&TrackInfo> {
        toc.sessions
            .iter()
            .flat_map(|s| s.tracks.iter())
            .filter(|t| t.track_type == TrackType::Audio)
            .collect()
    }

    /// Get all data tracks from TOC
    pub fn get_data_tracks(toc: &CdToc) -> Vec<&TrackInfo> {
        toc.sessions
            .iter()
            .flat_map(|s| s.tracks.iter())
            .filter(|t| matches!(t.track_type, TrackType::Data | TrackType::Mode1 | TrackType::Mode2))
            .collect()
    }

    /// Read a specific session from disc
    ///
    /// Each session may have a different filesystem (ISO 9660, HFS, etc.)
    pub fn read_session(drive_path: &str, session: u8) -> Result<Vec<u8>, DiskRipperError> {
        let toc = Self::read_toc(drive_path)?;

        let session_info = toc.sessions
            .iter()
            .find(|s| s.session_number == session)
            .ok_or_else(|| DiskRipperError::InvalidPath(format!("Session {} not found", session)))?;

        // Read all sectors in the session
        let sector_count = session_info.end_lba - session_info.start_lba;
        let mut data = Vec::with_capacity(sector_count as usize * 2048);

        // Placeholder: In production, this would read sectors from the drive
        warn!("Session reading not yet implemented");

        Ok(data)
    }

    /// Parse TOC from raw SCSI data
    ///
    /// TOC data format:
    /// - 2 bytes: TOC data length
    /// - 1 byte: First session number
    /// - 1 byte: Last session number
    /// - For each track:
    ///   - 1 byte: Reserved
    ///   - 1 byte: Control/ADR
    ///   - 1 byte: Track number
    ///   - 1 byte: Reserved
    ///   - 4 bytes: Track start address (MSF or LBA)
    pub fn parse_toc(raw_data: &[u8]) -> Result<CdToc, DiskRipperError> {
        if raw_data.len() < 4 {
            return Err(DiskRipperError::InvalidPath("TOC data too short".to_string()));
        }

        let toc_len = u16::from_be_bytes([raw_data[0], raw_data[1]]) as usize;
        let first_session = raw_data[2];
        let last_session = raw_data[3];

        let mut sessions: Vec<SessionInfo> = Vec::new();
        let mut current_session = first_session;
        let mut track_number = 1u8;

        // Parse track descriptors
        let mut offset = 4;
        while offset + 8 <= raw_data.len() && offset < toc_len + 2 {
            let control_adr = raw_data[offset + 1];
            let track = raw_data[offset + 2];
            let track_type = if (control_adr & 0x04) != 0 {
                TrackType::Data
            } else {
                TrackType::Audio
            };

            // Track start address (LBA in big-endian)
            let start_lba = u32::from_be_bytes([
                raw_data[offset + 4],
                raw_data[offset + 5],
                raw_data[offset + 6],
                raw_data[offset + 7],
            ]);

            // Find or create session
            if let Some(session) = sessions.iter_mut().find(|s| s.session_number == current_session) {
                session.tracks.push(TrackInfo {
                    track_number: track,
                    start_lba,
                    end_lba: 0, // Will be filled from next track
                    sector_count: 0,
                    track_type,
                    pre_emphasis: (control_adr & 0x01) != 0,
                    copy_permitted: (control_adr & 0x02) != 0,
                    channels: 2,
                    isrc: None,
                    title: None,
                });
                session.track_count += 1;
            } else {
                sessions.push(SessionInfo {
                    session_number: current_session,
                    start_lba,
                    end_lba: 0,
                    track_count: 1,
                    tracks: vec![TrackInfo {
                        track_number: track,
                        start_lba,
                        end_lba: 0,
                        sector_count: 0,
                        track_type,
                        pre_emphasis: (control_adr & 0x01) != 0,
                        copy_permitted: (control_adr & 0x02) != 0,
                        channels: 2,
                        isrc: None,
                        title: None,
                    }],
                });
            }

            offset += 8;
            track_number += 1;
        }

        // Calculate end LBAs
        for session in &mut sessions {
            for i in 0..session.tracks.len() {
                if i + 1 < session.tracks.len() {
                    session.tracks[i].end_lba = session.tracks[i + 1].start_lba;
                    session.tracks[i].sector_count =
                        (session.tracks[i].end_lba - session.tracks[i].start_lba) as u64;
                }
            }
            if let Some(first_track) = session.tracks.first() {
                session.start_lba = first_track.start_lba;
            }
        }

        let total_tracks = sessions.iter().map(|s| s.track_count).sum();

        Ok(CdToc {
            sessions,
            total_sessions: last_session - first_session + 1,
            total_tracks,
            lead_out_lba: 0,
        })
    }
}

/// Enhanced CD (Blue Book) support
///
/// Enhanced CDs contain both audio tracks and a data session.
/// The data session typically contains multimedia content.
pub struct EnhancedCdReader;

impl EnhancedCdReader {
    /// Detect if a CD is an enhanced CD
    pub fn is_enhanced_cd(toc: &CdToc) -> bool {
        // Enhanced CDs have at least 2 sessions
        // Session 1: Audio tracks
        // Session 2: Data track
        if toc.total_sessions < 2 {
            return false;
        }

        let first_session_audio = toc.sessions.first()
            .map(|s| s.tracks.iter().all(|t| t.track_type == TrackType::Audio))
            .unwrap_or(false);

        let last_session_data = toc.sessions.last()
            .map(|s| s.tracks.iter().all(|t| t.track_type != TrackType::Audio))
            .unwrap_or(false);

        first_session_audio && last_session_data
    }

    /// Get the data session from an enhanced CD
    pub fn get_data_session(toc: &CdToc) -> Option<&SessionInfo> {
        toc.sessions.iter().find(|s| {
            s.tracks.iter().all(|t| t.track_type != TrackType::Audio)
        })
    }

    /// Get the audio session from an enhanced CD
    pub fn get_audio_session(toc: &CdToc) -> Option<&SessionInfo> {
        toc.sessions.iter().find(|s| {
            s.tracks.iter().any(|t| t.track_type == TrackType::Audio)
        })
    }
}

/// CD-i (Compact Disc Interactive) support
pub struct CdIReader;

impl CdIReader {
    /// Detect if a CD is a CD-i disc
    pub fn is_cd_i(_toc: &CdToc) -> bool {
        // CD-i discs have a specific structure
        // Placeholder for now
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toc() {
        // Minimal TOC data
        let mut data = vec![0u8; 20];
        data[0] = 0; // TOC length high byte
        data[1] = 18; // TOC length low byte
        data[2] = 1; // First session
        data[3] = 1; // Last session
        // Track 1: Audio
        data[4] = 0; // Reserved
        data[5] = 0x10; // Control: audio, no pre-emphasis
        data[6] = 1; // Track number
        data[7] = 0; // Reserved
        data[8..12].copy_from_slice(&150u32.to_be_bytes()); // Start LBA
        // Track 2: Data
        data[12] = 0;
        data[13] = 0x14; // Control: data
        data[14] = 2;
        data[15] = 0;
        data[16..20].copy_from_slice(&1000u32.to_be_bytes());

        let toc = MultisessionReader::parse_toc(&data).unwrap();
        assert_eq!(toc.total_sessions, 1);
        assert_eq!(toc.total_tracks, 2);
        assert_eq!(toc.sessions[0].tracks[0].track_type, TrackType::Audio);
        assert_eq!(toc.sessions[0].tracks[1].track_type, TrackType::Data);
    }

    #[test]
    fn test_is_mixed_mode() {
        let toc = CdToc {
            sessions: vec![SessionInfo {
                session_number: 1,
                start_lba: 0,
                end_lba: 1000,
                track_count: 2,
                tracks: vec![
                    TrackInfo {
                        track_number: 1,
                        start_lba: 0,
                        end_lba: 100,
                        sector_count: 100,
                        track_type: TrackType::Data,
                        pre_emphasis: false,
                        copy_permitted: false,
                        channels: 2,
                        isrc: None,
                        title: None,
                    },
                    TrackInfo {
                        track_number: 2,
                        start_lba: 100,
                        end_lba: 1000,
                        sector_count: 900,
                        track_type: TrackType::Audio,
                        pre_emphasis: false,
                        copy_permitted: true,
                        channels: 2,
                        isrc: None,
                        title: None,
                    },
                ],
            }],
            total_sessions: 1,
            total_tracks: 2,
            lead_out_lba: 1000,
        };

        assert!(MultisessionReader::is_mixed_mode(&toc));
    }

    #[test]
    fn test_enhanced_cd_detection() {
        let toc = CdToc {
            sessions: vec![
                SessionInfo {
                    session_number: 1,
                    start_lba: 0,
                    end_lba: 1000,
                    track_count: 1,
                    tracks: vec![TrackInfo {
                        track_number: 1,
                        start_lba: 0,
                        end_lba: 1000,
                        sector_count: 1000,
                        track_type: TrackType::Audio,
                        pre_emphasis: false,
                        copy_permitted: true,
                        channels: 2,
                        isrc: None,
                        title: None,
                    }],
                },
                SessionInfo {
                    session_number: 2,
                    start_lba: 1000,
                    end_lba: 2000,
                    track_count: 1,
                    tracks: vec![TrackInfo {
                        track_number: 2,
                        start_lba: 1000,
                        end_lba: 2000,
                        sector_count: 1000,
                        track_type: TrackType::Data,
                        pre_emphasis: false,
                        copy_permitted: false,
                        channels: 2,
                        isrc: None,
                        title: None,
                    }],
                },
            ],
            total_sessions: 2,
            total_tracks: 2,
            lead_out_lba: 2000,
        };

        assert!(EnhancedCdReader::is_enhanced_cd(&toc));
        assert_eq!(EnhancedCdReader::get_audio_session(&toc).unwrap().session_number, 1);
        assert_eq!(EnhancedCdReader::get_data_session(&toc).unwrap().session_number, 2);
    }
}
