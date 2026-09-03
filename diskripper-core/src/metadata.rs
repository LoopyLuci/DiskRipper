//! CD-Text extraction and MusicBrainz/freedb metadata lookup.
//!
//! CD-Text is a storage format for album/track metadata stored in the R-W subchannels
//! of a compact disc. This module extracts CD-Text data and provides metadata lookup
//! from online databases.

use std::collections::HashMap;

use tracing::{info, warn};

use crate::error::DiskRipperError;

/// CD-Text pack types
const CDTEXT_PACK_TITLE: u8 = 0x80;
const CDTEXT_PACK_PERFORMER: u8 = 0x81;
const CDTEXT_PACK_SONGWRITER: u8 = 0x82;
const CDTEXT_PACK_COMPOSER: u8 = 0x83;
const CDTEXT_PACK_ARRANGER: u8 = 0x84;
const CDTEXT_PACK_MESSAGE: u8 = 0x85;
const CDTEXT_PACK_DISC_ID: u8 = 0x86;
const CDTEXT_PACK_GENRE: u8 = 0x87;
const CDTEXT_PACK_TOC_INFO: u8 = 0x88;
const CDTEXT_PACK_TOC_INFO2: u8 = 0x89;
const CDTEXT_PACK_RESERVED1: u8 = 0x8A;
const CDTEXT_PACK_RESERVED2: u8 = 0x8B;
const CDTEXT_PACK_RESERVED3: u8 = 0x8C;
const CDTEXT_PACK_RESERVED4: u8 = 0x8D;
const CDTEXT_PACK_UPC_EAN: u8 = 0x8E;
const CDTEXT_PACK_SIZE_INFO: u8 = 0x8F;

/// CD-Text information for a disc
#[derive(Debug, Clone, Default)]
pub struct CdTextInfo {
    pub title: Option<String>,
    pub performer: Option<String>,
    pub songwriter: Option<String>,
    pub composer: Option<String>,
    pub arranger: Option<String>,
    pub message: Option<String>,
    pub disc_id: Option<String>,
    pub genre: Option<String>,
    pub upc_ean: Option<String>,
    /// Per-track titles (track_number -> title)
    pub track_titles: HashMap<u8, String>,
    /// Per-track performers
    pub track_performers: HashMap<u8, String>,
}

impl CdTextInfo {
    /// Get title for a specific track, falling back to disc title
    pub fn get_track_title(&self, track_number: u8) -> Option<&str> {
        self.track_titles
            .get(&track_number)
            .map(|s| s.as_str())
            .or_else(|| self.title.as_deref())
    }

    /// Get performer for a specific track, falling back to disc performer
    pub fn get_track_performer(&self, track_number: u8) -> Option<&str> {
        self.track_performers
            .get(&track_number)
            .map(|s| s.as_str())
            .or_else(|| self.performer.as_deref())
    }
}

/// CD-Text parser
pub struct CdTextParser;

impl CdTextParser {
    /// Parse CD-Text data from raw subchannel data
    ///
    /// CD-Text data is stored in packs of 18 bytes each.
    /// Each pack contains:
    /// - Pack type (1 byte)
    /// - Track number (1 byte)
    /// - Sequential number (1 byte)
    /// - Character position (1 byte)
    /// - Data (12 bytes)
    /// - CRC (2 bytes)
    pub fn parse(raw_data: &[u8]) -> Result<CdTextInfo, DiskRipperError> {
        if raw_data.len() < 18 {
            return Err(DiskRipperError::InvalidPath("CD-Text data too short".to_string()));
        }

        let mut info = CdTextInfo::default();
        let num_packs = raw_data.len() / 18;

        for i in 0..num_packs {
            let offset = i * 18;
            let pack = &raw_data[offset..offset + 18];

            let pack_type = pack[0];
            let track_number = pack[1];
            let _sequential = pack[2];
            let _char_position = pack[3];
            let data = &pack[4..16];
            let _crc = &pack[16..18];

            // Decode text (ASCII or null-terminated)
            let text = String::from_utf8_lossy(data)
                .trim_end_matches('\0')
                .trim()
                .to_string();

            if text.is_empty() {
                continue;
            }

            match pack_type {
                CDTEXT_PACK_TITLE => {
                    if track_number == 0 {
                        info.title = Some(text);
                    } else {
                        info.track_titles.insert(track_number, text);
                    }
                }
                CDTEXT_PACK_PERFORMER => {
                    if track_number == 0 {
                        info.performer = Some(text);
                    } else {
                        info.track_performers.insert(track_number, text);
                    }
                }
                CDTEXT_PACK_SONGWRITER => {
                    if track_number == 0 {
                        info.songwriter = Some(text);
                    }
                }
                CDTEXT_PACK_COMPOSER => {
                    if track_number == 0 {
                        info.composer = Some(text);
                    }
                }
                CDTEXT_PACK_ARRANGER => {
                    if track_number == 0 {
                        info.arranger = Some(text);
                    }
                }
                CDTEXT_PACK_MESSAGE => {
                    if track_number == 0 {
                        info.message = Some(text);
                    }
                }
                CDTEXT_PACK_DISC_ID => {
                    if track_number == 0 {
                        info.disc_id = Some(text);
                    }
                }
                CDTEXT_PACK_GENRE => {
                    if track_number == 0 {
                        info.genre = Some(text);
                    }
                }
                CDTEXT_PACK_UPC_EAN => {
                    if track_number == 0 {
                        info.upc_ean = Some(text);
                    }
                }
                _ => {} // Unknown or reserved pack type
            }
        }

        info!(
            title = ?info.title,
            performer = ?info.performer,
            tracks = info.track_titles.len(),
            "Parsed CD-Text"
        );

        Ok(info)
    }

    /// Extract CD-Text from a CD using SCSI READ CD command with subchannel data
    ///
    /// This is a placeholder - actual implementation would use SCSI commands
    /// to read the R-W subchannels from the disc.
    pub fn extract_from_drive(_drive_path: &str) -> Result<CdTextInfo, DiskRipperError> {
        // Placeholder: In production, this would:
        // 1. Send SCSI READ CD command with subchannel type = 0x04 (CD-Text)
        // 2. Parse the returned data into CD-Text packs
        // 3. Call Self::parse() on the raw data

        warn!("CD-Text extraction from drive not yet implemented");
        Ok(CdTextInfo::default())
    }
}

/// MusicBrainz metadata lookup
pub struct MusicBrainzLookup;

impl MusicBrainzLookup {
    /// Look up disc metadata by disc ID
    ///
    /// MusicBrainz uses a calculated disc ID to query their database.
    /// The disc ID is based on the number of tracks and their frame offsets.
    pub async fn lookup_disc(_disc_id: &str) -> Result<MusicBrainzRelease, DiskRipperError> {
        // Placeholder: In production, this would:
        // 1. Query https://musicbrainz.org/ws/2/discid/{disc_id}
        // 2. Parse the XML/JSON response
        // 3. Return structured metadata

        warn!("MusicBrainz lookup not yet implemented");
        Err(DiskRipperError::UnsupportedDisc("MusicBrainz lookup not yet implemented".to_string()))
    }

    /// Calculate MusicBrainz disc ID from track offsets
    ///
    /// The disc ID is a hash of:
    /// - First track number
    /// - Last track number
    /// - Lead-out offset
    /// - Each track's offset
    pub fn calculate_disc_id(
        first_track: u8,
        last_track: u8,
        lead_out_offset: u32,
        track_offsets: &[u32],
    ) -> String {
        // MusicBrainz disc ID calculation
        // See: https://musicbrainz.org/doc/Disc_ID_Calculation

        let mut input = String::new();
        input.push_str(&format!("{:02}", first_track));
        input.push_str(&format!("{:02}", last_track));
        input.push_str(&format!("{:08X}", lead_out_offset));

        for offset in track_offsets {
            input.push_str(&format!("{:08X}", offset));
        }

        // SHA-1 hash, then base64 encode
        use sha1::{Sha1, Digest};
        let mut hasher = Sha1::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();

        // Convert to base64 and replace special characters
        use base64::Engine as _;
        let base64 = base64::engine::general_purpose::STANDARD.encode(&hash);
        base64.replace('+', ".").replace('/', "_").replace('=', "-")
    }
}

/// FreeDB metadata lookup
pub struct FreedbLookup;

impl FreedbLookup {
    /// Look up disc metadata by disc ID
    ///
    /// FreeDB is a free alternative to CDDB for CD metadata.
    pub async fn lookup_disc(
        _disc_id: &str,
        _category: Option<&str>,
    ) -> Result<FreedbEntry, DiskRipperError> {
        // Placeholder: In production, this would:
        // 1. Query https://gnudb.org/~cddb/cddb.cgi
        // 2. Parse the response
        // 3. Return structured metadata

        warn!("FreeDB lookup not yet implemented");
        Err(DiskRipperError::UnsupportedDisc("FreeDB lookup not yet implemented".to_string()))
    }

    /// Calculate FreeDB disc ID
    ///
    /// FreeDB disc ID is a hash of the track offsets and total length.
    pub fn calculate_disc_id(
        track_offsets: &[u32],
        total_length_seconds: u32,
    ) -> String {
        // FreeDB disc ID calculation
        // See: https://ftp.freedb.org/pub/freedb/latest/CDDBPROTO

        let mut sum: u32 = 0;
        for offset in track_offsets {
            let mut n = *offset;
            while n > 0 {
                sum += n % 10;
                n /= 10;
            }
        }

        let total_length_frames = total_length_seconds * 75;
        let disc_id = ((sum % 0xFF) << 24) | (total_length_frames << 8) | (track_offsets.len() as u32);

        format!("{:08x}", disc_id)
    }
}

/// Combined metadata provider that tries multiple sources
pub struct MetadataProvider;

impl MetadataProvider {
    /// Look up metadata from all available sources
    ///
    /// Tries in order:
    /// 1. CD-Text (from disc)
    /// 2. MusicBrainz (online)
    /// 3. FreeDB (online)
    pub async fn lookup_metadata(
        cdtext: Option<&CdTextInfo>,
        disc_id: &str,
    ) -> Result<DiscMetadata, DiskRipperError> {
        // First try CD-Text
        if let Some(cdtext) = cdtext {
            if cdtext.title.is_some() || !cdtext.track_titles.is_empty() {
                return Ok(DiscMetadata {
                    title: cdtext.title.clone(),
                    artist: cdtext.performer.clone(),
                    genre: cdtext.genre.clone(),
                    year: None,
                    track_count: cdtext.track_titles.len() as u8,
                    source: "CD-Text".to_string(),
                });
            }
        }

        // Try MusicBrainz
        match MusicBrainzLookup::lookup_disc(disc_id).await {
            Ok(release) => {
                return Ok(DiscMetadata {
                    title: Some(release.title),
                    artist: release.artist,
                    genre: None,
                    year: release.year,
                    track_count: release.track_count,
                    source: "MusicBrainz".to_string(),
                });
            }
            Err(_) => {}
        }

        // Try FreeDB
        match FreedbLookup::lookup_disc(disc_id, None).await {
            Ok(entry) => {
                return Ok(DiscMetadata {
                    title: Some(entry.title),
                    artist: Some(entry.artist),
                    genre: entry.genre,
                    year: None,
                    track_count: entry.track_count,
                    source: "FreeDB".to_string(),
                });
            }
            Err(_) => {}
        }

        Err(DiskRipperError::Io("No metadata found".to_string()))
    }
}

/// Combined disc metadata
#[derive(Debug, Clone)]
pub struct DiscMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_count: u8,
    pub source: String,
}

/// MusicBrainz release information
#[derive(Debug, Clone)]
pub struct MusicBrainzRelease {
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub track_count: u8,
}

/// FreeDB entry information
#[derive(Debug, Clone)]
pub struct FreedbEntry {
    pub title: String,
    pub artist: String,
    pub genre: Option<String>,
    pub track_count: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdtext_parse() {
        // Create a minimal CD-Text pack for title
        let mut data = vec![0u8; 18];
        data[0] = CDTEXT_PACK_TITLE;
        data[1] = 0; // Track 0 = disc title
        let title = b"Test Album";
        data[4..4 + title.len()].copy_from_slice(title);

        let info = CdTextParser::parse(&data).unwrap();
        assert_eq!(info.title, Some("Test Album".to_string()));
    }

    #[test]
    fn test_cdtext_track_title() {
        let mut data = vec![0u8; 36]; // Two packs
        // Disc title
        data[0] = CDTEXT_PACK_TITLE;
        data[1] = 0;
        let title = b"Album Name";
        data[4..4 + title.len()].copy_from_slice(title);
        // Track 1 title
        data[18] = CDTEXT_PACK_TITLE;
        data[19] = 1; // Track 1
        let track_title = b"Track One";
        data[22..22 + track_title.len()].copy_from_slice(track_title);

        let info = CdTextParser::parse(&data).unwrap();
        assert_eq!(info.title, Some("Album Name".to_string()));
        assert_eq!(info.track_titles.get(&1), Some(&"Track One".to_string()));
        assert_eq!(info.get_track_title(1), Some("Track One"));
        assert_eq!(info.get_track_title(2), Some("Album Name")); // Fallback
    }

    #[test]
    fn test_freedb_disc_id() {
        let offsets = vec![150, 2000, 4000];
        let total_length = 120;
        let disc_id = FreedbLookup::calculate_disc_id(&offsets, total_length);
        assert_eq!(disc_id.len(), 8);
    }
}
