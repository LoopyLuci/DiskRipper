use std::path::Path;

use crate::error::DiskRipperError;

/// DVD IFO parser for title/chapter extraction
#[derive(Debug)]
pub struct DvdParser;

/// DVD title information
#[derive(Debug, Clone)]
pub struct DvdTitle {
    pub title_number: u16,
    pub chapters: Vec<DvdChapter>,
    pub duration_seconds: f64,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub sector_count: u64,
}

/// DVD chapter information
#[derive(Debug, Clone)]
pub struct DvdChapter {
    pub chapter_number: u16,
    pub start_cell: u8,
    pub end_cell: u8,
    pub duration_seconds: f64,
    pub start_sector: u64,
    pub end_sector: u64,
}

/// Audio track information
#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub track_number: u8,
    pub codec: AudioCodec,
    pub sample_rate: u32,
    pub channels: u8,
    pub language: String,
    pub stream_id: u8,
}

/// Audio codec types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    Ac3,
    Dts,
    Lpcm,
    Mpeg1,
    Mpeg2,
    Sdds,
    Unknown(u8),
}

impl From<u8> for AudioCodec {
    fn from(byte: u8) -> Self {
        match byte & 0x07 {
            0 => AudioCodec::Ac3,
            1 => AudioCodec::Dts,
            2 => AudioCodec::Lpcm,
            3 => AudioCodec::Mpeg1,
            4 => AudioCodec::Mpeg2,
            5 => AudioCodec::Sdds,
            _ => AudioCodec::Unknown(byte),
        }
    }
}

/// Subtitle track information
#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    pub track_number: u8,
    pub language: String,
    pub stream_id: u8,
}

/// VTS (Video Title Set) information from IFO
#[derive(Debug, Clone)]
pub struct VtsInfo {
    pub vts_number: u16,
    pub sector_count: u64,
    pub chapter_count: u16,
    pub audio_track_count: u8,
    pub subtitle_track_count: u8,
}

impl DvdParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse DVD IFO file and extract title/chapter information
    pub fn parse_ifo(&self, ifo_path: &Path) -> Result<Vec<DvdTitle>, DiskRipperError> {
        let data = std::fs::read(ifo_path)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read IFO: {}", e)))?;
        
        if data.len() < 0x100 {
            return Err(DiskRipperError::InvalidPath("IFO file too short".to_string()));
        }
        
        // Check for DVD-Video identifier
        let identifier = &data[0..12];
        if identifier != b"DVDVIDEO-VTS" && identifier != b"DVDVIDEO-VMG" {
            return Err(DiskRipperError::InvalidPath("Not a valid DVD IFO file".to_string()));
        }
        
        let is_vmg = identifier == b"DVDVIDEO-VMG";
        
        if is_vmg {
            self.parse_vmg(&data)
        } else {
            self.parse_vts(&data).map(|v| vec![v])
        }
    }

    /// Parse VMG (Video Manager) IFO - contains all titles
    fn parse_vmg(&self, data: &[u8]) -> Result<Vec<DvdTitle>, DiskRipperError> {
        let mut titles = Vec::new();
        
        // VMG structure:
        // 0x000-0x00F: Identifier "DVDVIDEO-VMG"
        // 0x020: Last sector of VMG
        // 0x0C4: Start sector of VMG IFO
        // 0x0E0: Start sector of VTS_PTT_SRPT (title search pointer table)
        // 0x0E4: Number of titles
        // 0x100: VMG_MAT (Video Manager Information Management Table)
        
        let title_count = u16::from_be_bytes([data[0x0E4], data[0x0E5]]) as usize;
        let vts_table_sector = u32::from_be_bytes([data[0x0C4], data[0x0C5], data[0x0C6], data[0x0C7]]);
        
        // Parse title table
        let title_table_offset = 0x0F00; // Typically at sector 0x0F00 from start
        if data.len() > title_table_offset + title_count * 4 {
            for i in 0..title_count.min(99) {
                let offset = title_table_offset + i * 4;
                let vts_number = u16::from_be_bytes([data[offset], data[offset + 1]]);
                let vts_sector = u32::from_be_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);
                
                titles.push(DvdTitle {
                    title_number: vts_number,
                    chapters: Vec::new(),
                    duration_seconds: 0.0,
                    audio_tracks: Vec::new(),
                    subtitle_tracks: Vec::new(),
                    sector_count: 0,
                });
            }
        }
        
        Ok(titles)
    }

    /// Parse VTS (Video Title Set) IFO - single title
    fn parse_vts(&self, data: &[u8]) -> Result<DvdTitle, DiskRipperError> {
        let vts_number = u16::from_be_bytes([data[0x000], data[0x001]]);
        
        // VTS structure:
        // 0x000-0x00F: Identifier "DVDVIDEO-VTS"
        // 0x020: Last sector of VTS
        // 0x080: End byte address of VTS_MAT
        // 0x0C0: Start sector of VTS_VOBS (Video Object Set)
        // 0x0C4: Start sector of VTSM_VOBS (Menu VOBS)
        // 0x0C8: Start sector of VTSTT_VOBS (Title VOBS)
        // 0x100: VTS_MAT (VTS Management Table)
        
        let sector_count = u32::from_be_bytes([data[0x020], data[0x021], data[0x022], data[0x023]]) as u64;
        
        // Parse VTS_MAT at offset 0x100
        let vts_mat_offset = 0x100;
        if data.len() < vts_mat_offset + 0x100 {
            return Err(DiskRipperError::InvalidPath("VTS_MAT too short".to_string()));
        }
        
        let mat = &data[vts_mat_offset..];
        
        // Audio stream count at offset 0x200 (2 bytes)
        let audio_track_count = u16::from_be_bytes([mat[0x200], mat[0x201]]) as u8;
        
        // Sub-picture stream count at offset 0x202 (2 bytes)
        let subtitle_track_count = u16::from_be_bytes([mat[0x202], mat[0x203]]) as u8;
        
        // Parse audio tracks
        let mut audio_tracks = Vec::new();
        let audio_attr_offset = 0x204;
        for i in 0..audio_track_count.min(8) {
            let offset = audio_attr_offset + i as usize * 8;
            if offset + 8 <= mat.len() {
                let codec_byte = mat[offset];
                let stream_id = mat[offset + 1] & 0x07;
                let channels = ((mat[offset + 2] >> 4) & 0x0F) as u8 + 1;
                let sample_rate = match mat[offset + 2] & 0x0F {
                    0 => 48000,
                    1 => 96000,
                    _ => 48000,
                };
                
                audio_tracks.push(AudioTrack {
                    track_number: i + 1,
                    codec: AudioCodec::from(codec_byte),
                    sample_rate,
                    channels,
                    language: Self::parse_language_code(&mat[offset + 4..offset + 8]),
                    stream_id,
                });
            }
        }
        
        // Parse subtitle tracks
        let mut subtitle_tracks = Vec::new();
        let subtitle_attr_offset = audio_attr_offset + audio_track_count as usize * 8;
        for i in 0..subtitle_track_count.min(32) {
            let offset = subtitle_attr_offset + i as usize * 8;
            if offset + 8 <= mat.len() {
                let stream_id = mat[offset] & 0x1F;
                subtitle_tracks.push(SubtitleTrack {
                    track_number: i + 1,
                    language: Self::parse_language_code(&mat[offset + 2..offset + 6]),
                    stream_id,
                });
            }
        }
        
        // Parse chapter information from PGC (Program Chain)
        let chapters = self.parse_chapters(data, vts_mat_offset)?;
        
        // Calculate total duration
        let duration_seconds = chapters.iter().map(|c| c.duration_seconds).sum();
        
        Ok(DvdTitle {
            title_number: vts_number,
            chapters,
            duration_seconds,
            audio_tracks,
            subtitle_tracks,
            sector_count,
        })
    }

    /// Parse chapter information from PGC
    fn parse_chapters(&self, data: &[u8], vts_mat_offset: usize) -> Result<Vec<DvdChapter>, DiskRipperError> {
        let mut chapters = Vec::new();
        
        let mat = &data[vts_mat_offset..];
        
        // PGC category at offset 0x200 (4 bytes)
        let pgc_category = u32::from_be_bytes([mat[0x200], mat[0x201], mat[0x202], mat[0x203]]);
        let title_pgc_count = (pgc_category & 0xFFFF) as u16;
        
        // Cell playback table at offset 0xE4 (4 bytes)
        let cell_playback_table_sector = u32::from_be_bytes([mat[0xE4], mat[0xE5], mat[0xE6], mat[0xE7]]);
        
        // Parse each PGC
        let pgc_offset = vts_mat_offset + 0xE8;
        for i in 0..title_pgc_count.min(99) {
            let offset = pgc_offset + i as usize * 8;
            if offset + 8 > data.len() {
                break;
            }
            
            let pgc_sector = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
            let pgc_length = u16::from_be_bytes([data[offset + 4], data[offset + 5]]) as usize;
            
            if pgc_sector == 0 || pgc_length == 0 {
                continue;
            }
            
            let pgc_data_offset = pgc_sector as usize * 2048;
            if pgc_data_offset + pgc_length > data.len() {
                continue;
            }
            
            let pgc = &data[pgc_data_offset..pgc_data_offset + pgc_length];
            
            // PGC header at offset 0x00
            // Number of programs: byte 0x02
            // Number of cells: byte 0x03
            if pgc.len() < 0x0E {
                continue;
            }
            
            let program_count = pgc[0x02] as u16;
            let cell_count = pgc[0x03] as u16;
            
            // Playback time at offset 0x04 (4 bytes BCD)
            let playback_time = Self::parse_bcd_time(&pgc[0x04..0x08]);
            
            // Cell start sectors from cell address table
            let cell_table_offset = 0xEC;
            for j in 0..cell_count.min(255) {
                let cell_offset = cell_table_offset + j as usize * 4;
                if cell_offset + 4 > pgc.len() {
                    break;
                }
                
                let start_sector = u32::from_be_bytes([
                    pgc[cell_offset],
                    pgc[cell_offset + 1],
                    pgc[cell_offset + 2],
                    pgc[cell_offset + 3],
                ]) as u64;
                
                let end_sector = if j + 1 < cell_count {
                    let next_offset = cell_table_offset + (j + 1) as usize * 4;
                    if next_offset + 4 <= pgc.len() {
                        u32::from_be_bytes([
                            pgc[next_offset],
                            pgc[next_offset + 1],
                            pgc[next_offset + 2],
                            pgc[next_offset + 3],
                        ]) as u64
                    } else {
                        start_sector + 1000
                    }
                } else {
                    start_sector + 1000
                };
                
                chapters.push(DvdChapter {
                    chapter_number: j + 1,
                    start_cell: j as u8 + 1,
                    end_cell: j as u8 + 1,
                    duration_seconds: playback_time / program_count.max(1) as f64,
                    start_sector,
                    end_sector,
                });
            }
        }
        
        Ok(chapters)
    }

    /// Parse BCD-encoded time (DVD format)
    fn parse_bcd_time(data: &[u8]) -> f64 {
        if data.len() < 4 {
            return 0.0;
        }
        
        // BCD format: HH:MM:SS:FF
        let hours = (data[0] >> 4) * 10 + (data[0] & 0x0F);
        let minutes = (data[1] >> 4) * 10 + (data[1] & 0x0F);
        let seconds = (data[2] >> 4) * 10 + (data[2] & 0x0F);
        let frames = (data[3] >> 4) * 10 + (data[3] & 0x0F);
        
        hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds as f64 + frames as f64 / 75.0
    }

    /// Parse language code from 4-byte field
    fn parse_language_code(data: &[u8]) -> String {
        if data.len() < 4 {
            return "und".to_string();
        }
        
        // Language code is typically 2 ASCII characters at offset 0-1
        let lang = &data[0..2];
        if lang[0].is_ascii_alphabetic() && lang[1].is_ascii_alphabetic() {
            String::from_utf8_lossy(lang).to_string().to_lowercase()
        } else {
            "und".to_string()
        }
    }

    /// Get VTS information summary
    pub fn get_vts_info(&self, ifo_path: &Path) -> Result<VtsInfo, DiskRipperError> {
        let data = std::fs::read(ifo_path)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read IFO: {}", e)))?;
        
        if data.len() < 0x100 {
            return Err(DiskRipperError::InvalidPath("IFO file too short".to_string()));
        }
        
        let vts_number = u16::from_be_bytes([data[0x000], data[0x001]]);
        let sector_count = u32::from_be_bytes([data[0x020], data[0x021], data[0x022], data[0x023]]) as u64;
        
        let vts_mat_offset = 0x100;
        if data.len() < vts_mat_offset + 0x204 {
            return Err(DiskRipperError::InvalidPath("VTS_MAT too short".to_string()));
        }
        
        let mat = &data[vts_mat_offset..];
        let audio_track_count = u16::from_be_bytes([mat[0x200], mat[0x201]]) as u8;
        let subtitle_track_count = u16::from_be_bytes([mat[0x202], mat[0x203]]) as u8;
        
        // Chapter count from PGC
        let pgc_category = u32::from_be_bytes([mat[0x200], mat[0x201], mat[0x202], mat[0x203]]);
        let chapter_count = (pgc_category & 0xFFFF) as u16;
        
        Ok(VtsInfo {
            vts_number,
            sector_count,
            chapter_count,
            audio_track_count,
            subtitle_track_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_bcd_time() {
        // 01:30:45:20 = 1 hour, 30 min, 45 sec, 20 frames
        let data = [0x01, 0x30, 0x45, 0x20];
        let time = DvdParser::parse_bcd_time(&data);
        assert!((time - 5445.267).abs() < 0.01);
    }
    
    #[test]
    fn test_parse_language_code() {
        let data = *b"en\x00\x00";
        assert_eq!(DvdParser::parse_language_code(&data), "en");
        
        let data = *b"fr\x00\x00";
        assert_eq!(DvdParser::parse_language_code(&data), "fr");
        
        let data = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(DvdParser::parse_language_code(&data), "und");
    }
    
    #[test]
    fn test_audio_codec_from() {
        assert_eq!(AudioCodec::from(0x00), AudioCodec::Ac3);
        assert_eq!(AudioCodec::from(0x01), AudioCodec::Dts);
        assert_eq!(AudioCodec::from(0x02), AudioCodec::Lpcm);
        assert_eq!(AudioCodec::from(0x03), AudioCodec::Mpeg1);
        assert_eq!(AudioCodec::from(0x04), AudioCodec::Mpeg2);
        assert_eq!(AudioCodec::from(0x05), AudioCodec::Sdds);
    }
}
