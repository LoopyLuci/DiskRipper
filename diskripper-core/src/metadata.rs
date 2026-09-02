use serde::{Deserialize, Serialize};

/// MusicBrainz metadata response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzRelease {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub date: Option<String>,
    pub tracks: Vec<MusicBrainzTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzTrack {
    pub number: u32,
    pub title: String,
    pub length: Option<u32>, // milliseconds
    pub artist: Option<String>,
}

/// freedb metadata response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreedbEntry {
    pub category: String,
    pub disc_id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<String>,
    pub genre: String,
    pub tracks: Vec<FreedbTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreedbTrack {
    pub title: String,
    pub length: f64, // seconds
    pub offset: u32, // sectors
}

/// CD-Text data extracted from subchannel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdText {
    pub title: Option<String>,
    pub performer: Option<String>,
    pub songwriter: Option<String>,
    pub composer: Option<String>,
    pub arranger: Option<String>,
    pub message: Option<String>,
    pub genre: Option<String>,
    pub upc_ean: Option<String>,
    pub tracks: Vec<CdTextTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdTextTrack {
    pub track_number: u8,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub isrc: Option<String>, // International Standard Recording Code
}

/// ISRC (International Standard Recording Code)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Isrc {
    pub country_code: String,   // 2 chars
    pub registrant_code: String, // 3 chars
    pub year: String,           // 2 chars
    pub designation: String,    // 5 chars
}

impl Isrc {
    pub fn to_string(&self) -> String {
        format!("{}{}{}{}", self.country_code, self.registrant_code, self.year, self.designation)
    }
    
    pub fn from_raw(raw: &[u8]) -> Option<Self> {
        if raw.len() < 12 {
            return None;
        }
        
        // ISRC is stored as ASCII in subchannel
        let isrc_str = String::from_utf8_lossy(raw);
        let parts: Vec<&str> = isrc_str.split('-').collect();
        
        if parts.len() == 4 {
            Some(Isrc {
                country_code: parts[0].to_string(),
                registrant_code: parts[1].to_string(),
                year: parts[2].to_string(),
                designation: parts[3].to_string(),
            })
        } else if isrc_str.len() >= 12 {
            Some(Isrc {
                country_code: isrc_str[0..2].to_string(),
                registrant_code: isrc_str[2..5].to_string(),
                year: isrc_str[5..7].to_string(),
                designation: isrc_str[7..12].to_string(),
            })
        } else {
            None
        }
    }
}

/// AccurateRip checksum for a track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccurateRipChecksum {
    pub track_number: u32,
    pub crc: u32, // CRC32 of first and last sectors
    pub offset_crc: Option<u32>, // CRC32 with drive offset applied
}

/// AccurateRip verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccurateRipResult {
    pub track_number: u32,
    pub confidence: u32, // Number of matching rips
    pub is_secure: bool, // Confidence >= 2
    pub match_found: bool,
}

/// Metadata provider trait for extensibility
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, disc_id: &str) -> Result<Vec<MetadataResult>, MetadataError>;
}

#[derive(Debug, Clone)]
pub struct MetadataResult {
    pub source: String,
    pub release_id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<String>,
    pub tracks: Vec<MetadataTrackResult>,
}

#[derive(Debug, Clone)]
pub struct MetadataTrackResult {
    pub track_number: u32,
    pub title: String,
    pub length_seconds: Option<f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Not found")]
    NotFound,
    #[error("Rate limited")]
    RateLimited,
}

/// Disc ID calculation
pub struct DiscId;

impl DiscId {
    /// Calculate freedb disc ID from TOC
    pub fn calculate_freedb_id(toc: &[u64]) -> u32 {
        let mut checksum: u32 = 0;
        
        for &offset in toc {
            let mut frames = offset;
            while frames > 0 {
                checksum += (frames % 10) as u32;
                frames /= 10;
            }
        }
        
        // Add lead-out offset
        if !toc.is_empty() {
            let lead_out = toc[toc.len() - 1] + 150; // Add 2-second lead-out
            let mut frames = lead_out;
            while frames > 0 {
                checksum += (frames % 10) as u32;
                frames /= 10;
            }
        }
        
        checksum % 0xFF
    }
    
    /// Calculate MusicBrainz disc ID from TOC
    pub fn calculate_musicbrainz_id(toc: &[u64]) -> String {
        use sha1::{Sha1, Digest};
        
        let mut hasher = Sha1::new();
        
        // First track number (1)
        hasher.update(&[1]);
        
        // Last track number
        if !toc.is_empty() {
            let last_track = std::cmp::min(toc.len() as u8, 99);
            hasher.update(&[last_track]);
        }
        
        // Lead-out offset (disc length in frames)
        if !toc.is_empty() {
            let lead_out = toc[toc.len() - 1] + 150;
            hasher.update(&(lead_out as u32).to_be_bytes());
        }
        
        // Track offsets
        for &offset in toc {
            hasher.update(&(offset as u32).to_be_bytes());
        }
        
        let result = hasher.finalize();
        result.as_slice().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// AccurateRip calculator
pub struct AccurateRip;

impl AccurateRip {
    /// Calculate CRC32 for track (first 5 sectors + last 5 sectors)
    pub fn calculate_track_crc(data: &[u8], track_start: u64, track_length: u64) -> u32 {
        let sector_size = 2352usize;
        let start = (track_start as usize) * sector_size;
        let end = start + (track_length as usize) * sector_size;
        
        if end > data.len() {
            return 0;
        }
        
        let mut crc: u32 = 0xFFFFFFFF;
        
        // First 5 sectors
        let first_end = std::cmp::min(start + 5 * sector_size, data.len());
        for byte in &data[start..first_end] {
            crc = crc32_update(crc, *byte);
        }
        
        // Last 5 sectors
        let last_start = if end > 5 * sector_size { end - 5 * sector_size } else { start };
        for byte in &data[last_start..end] {
            crc = crc32_update(crc, *byte);
        }
        
        crc ^ 0xFFFFFFFF
    }
    
    /// Calculate CRC32 for entire track (for non-AccurateRip verification)
    pub fn calculate_full_crc(data: &[u8], track_start: u64, track_length: u64) -> u32 {
        let sector_size = 2352usize;
        let start = (track_start as usize) * sector_size;
        let end = start + (track_length as usize) * sector_size;
        
        if end > data.len() {
            return 0;
        }
        
        let mut crc: u32 = 0xFFFFFFFF;
        for byte in &data[start..end] {
            crc = crc32_update(crc, *byte);
        }
        
        crc ^ 0xFFFFFFFF
    }
}

fn crc32_update(crc: u32, byte: u8) -> u32 {
    let mut crc = crc ^ (byte as u32);
    for _ in 0..8 {
        if crc & 1 != 0 {
            crc = (crc >> 1) ^ 0xEDB88320;
        } else {
            crc >>= 1;
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freedb_id_calculation() {
        let toc = vec![0u64, 1000, 2000, 3000];
        let id = DiscId::calculate_freedb_id(&toc);
        assert!(id < 256);
    }

    #[test]
    fn test_musicbrainz_id_calculation() {
        let toc = vec![0u64, 1000, 2000, 3000];
        let id = DiscId::calculate_musicbrainz_id(&toc);
        assert!(!id.is_empty());
        assert!(id.len() <= 40);
    }

    #[test]
    fn test_crc32_calculation() {
        let data = vec![0xFFu8; 2352 * 10];
        let crc = AccurateRip::calculate_track_crc(&data, 0, 10);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_isrc_parsing() {
        let raw = b"USRC17607839";
        let isrc = Isrc::from_raw(raw).unwrap();
        assert_eq!(isrc.country_code, "US");
        assert_eq!(isrc.registrant_code, "RC1");
        assert_eq!(isrc.year, "76");
        assert_eq!(isrc.designation, "07839");
    }
}
