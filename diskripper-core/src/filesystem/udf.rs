#![allow(dead_code)]

use std::collections::HashMap;

use crate::error::DiskRipperError;
use crate::types::FileEntry;
use super::{FilesystemReader, FilesystemType, VolumeInfo};

/// UDF descriptor tag
#[derive(Debug, Clone)]
pub struct DescriptorTag {
    pub tag_identifier: u16,
    pub descriptor_version: u16,
    pub tag_serial_number: u16,
    pub descriptor_crc: u16,
    pub descriptor_crc_length: u16,
    pub tag_location: u32,
}

/// UDF Entity Identifier
#[derive(Debug, Clone)]
pub struct EntityIdentifier {
    pub flags: u8,
    pub identifier: String,
    pub identifier_suffix: String,
}

/// UDF ICB (Information Control Block)
#[derive(Debug, Clone)]
pub struct ICB {
    pub prior_recorded_number_of_direct_entries: u32,
    pub strategy_type: u16,
    pub strategy_parameters: Vec<u8>,
    pub maximum_number_of_entries: u16,
    pub file_type: u8,
    pub parent_icb_location: (u32, u16),
    pub flags: u16,
}

/// UDF File Identifier Descriptor
#[derive(Debug, Clone)]
pub struct FileIdentifierDescriptor {
    pub volume_descriptor_sequence_number: u32,
    pub file_characteristics: u8,
    pub length_of_file_identifier: u8,
    pub icb: ICB,
    pub length_of_implementation_use: u16,
    pub file_identifier: String,
}

/// UDF File Entry Descriptor
#[derive(Debug, Clone)]
pub struct FileEntryDescriptor {
    pub icb: ICB,
    pub uid: u32,
    pub gid: u32,
    pub permissions: u32,
    pub file_link_count: u16,
    pub information_length: u64,
    pub logical_blocks_recorded: u64,
    pub unique_id: u64,
}

/// UDF reader implementation
pub struct UdfReader {
    data: Vec<u8>,
    block_size: u32,
    anchor_volume_descriptor_pointer: Option<u32>,
    root_entries: HashMap<String, FileEntry>,
}

impl UdfReader {
    pub fn new(data: Vec<u8>) -> Result<Self, DiskRipperError> {
        if data.len() < 0x10000 {
            return Err(DiskRipperError::InvalidPath(
                "Data too small for UDF".to_string(),
            ));
        }

        if &data[0x8001..0x8006] != b"NSR02" && &data[0x8001..0x8006] != b"NSR03" {
            return Err(DiskRipperError::InvalidPath(
                "Not a valid UDF image".to_string(),
            ));
        }

        Ok(Self {
            data,
            block_size: 2048,
            anchor_volume_descriptor_pointer: None,
            root_entries: HashMap::new(),
        })
    }

    /// Parse Anchor Volume Descriptor Pointer (AVDP)
    fn parse_anchor_vdp(&mut self, offset: u32) -> Result<(), DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 512 {
            return Err(DiskRipperError::InvalidPath("AVDP too short".to_string()));
        }

        let tag = self.parse_descriptor_tag(&data[0..16])?;
        if tag.tag_identifier != 2 {
            return Err(DiskRipperError::InvalidPath("Invalid AVDP tag".to_string()));
        }

        let main_vdse = self.parse_extent_ad(&data[16..28]);
        self.parse_volume_descriptor_sequence(main_vdse.0, main_vdse.1)?;
        self.anchor_volume_descriptor_pointer = Some(offset);

        Ok(())
    }

    /// Parse Volume Descriptor Sequence
    fn parse_volume_descriptor_sequence(
        &mut self,
        start_lba: u32,
        length: u32,
    ) -> Result<(), DiskRipperError> {
        let mut offset = start_lba * self.block_size;
        let end = offset + length;

        while offset < end && (offset as usize) < self.data.len() {
            if (offset as usize) + 16 > self.data.len() {
                break;
            }
            let tag = self.parse_descriptor_tag(&self.data[offset as usize..])?;

            if tag.tag_identifier == 8 {
                break;
            }

            offset += self.block_size;
        }

        Ok(())
    }

    /// Parse Descriptor Tag
    fn parse_descriptor_tag(&self, data: &[u8]) -> Result<DescriptorTag, DiskRipperError> {
        if data.len() < 16 {
            return Err(DiskRipperError::InvalidPath(
                "Descriptor tag too short".to_string(),
            ));
        }

        Ok(DescriptorTag {
            tag_identifier: u16::from_le_bytes([data[0], data[1]]),
            descriptor_version: u16::from_le_bytes([data[2], data[3]]),
            tag_serial_number: u16::from_le_bytes([data[4], data[5]]),
            descriptor_crc: u16::from_le_bytes([data[6], data[7]]),
            descriptor_crc_length: u16::from_le_bytes([data[8], data[9]]),
            tag_location: u32::from_le_bytes([data[10], data[11], data[12], data[13]]),
        })
    }

    /// Parse Extent Address Descriptor (EAD)
    fn parse_extent_ad(&self, data: &[u8]) -> (u32, u32) {
        if data.len() < 8 {
            return (0, 0);
        }
        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let location = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        (location, length)
    }

    /// Parse Entity Identifier
    fn parse_entity_identifier(&self, data: &[u8]) -> EntityIdentifier {
        EntityIdentifier {
            flags: data[0],
            identifier: String::from_utf8_lossy(&data[1..23]).to_string(),
            identifier_suffix: String::from_utf8_lossy(&data[23..]).to_string(),
        }
    }

    /// Parse ICB (Information Control Block)
    fn parse_icb(&self, data: &[u8]) -> Result<ICB, DiskRipperError> {
        if data.len() < 19 {
            return Err(DiskRipperError::InvalidPath("ICB too short".to_string()));
        }

        Ok(ICB {
            prior_recorded_number_of_direct_entries: u32::from_le_bytes([
                data[0], data[1], data[2], data[3],
            ]),
            strategy_type: u16::from_le_bytes([data[4], data[5]]),
            strategy_parameters: data[6..8].to_vec(),
            maximum_number_of_entries: u16::from_le_bytes([data[8], data[9]]),
            file_type: data[10],
            parent_icb_location: (
                u32::from_le_bytes([data[11], data[12], data[13], data[14]]),
                u16::from_le_bytes([data[15], data[16]]),
            ),
            flags: u16::from_le_bytes([data[17], data[18]]),
        })
    }

    /// Parse File Identifier Descriptor
    fn parse_file_identifier(
        &self,
        data: &[u8],
    ) -> Result<FileIdentifierDescriptor, DiskRipperError> {
        if data.len() < 38 {
            return Err(DiskRipperError::InvalidPath("FID too short".to_string()));
        }

        let tag = self.parse_descriptor_tag(&data[0..16])?;
        let file_characteristics = data[18];
        let length_of_file_identifier = data[19];
        let icb = self.parse_icb(&data[20..36])?;
        let length_of_implementation_use = u16::from_le_bytes([data[36], data[37]]);

        let fid_start = 38 + length_of_implementation_use as usize;
        let fid_end = fid_start + length_of_file_identifier as usize;

        let file_identifier = if fid_end <= data.len() {
            String::from_utf8_lossy(&data[fid_start..fid_end]).to_string()
        } else {
            String::new()
        };

        Ok(FileIdentifierDescriptor {
            volume_descriptor_sequence_number: tag.tag_location,
            file_characteristics,
            length_of_file_identifier,
            icb,
            length_of_implementation_use,
            file_identifier,
        })
    }

    /// Parse directory contents
    fn parse_directory(&self, data: &[u8]) -> Result<Vec<FileEntry>, DiskRipperError> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos + 16 <= data.len() {
            let tag = match self.parse_descriptor_tag(&data[pos..]) {
                Ok(t) => t,
                Err(_) => break,
            };

            if tag.tag_identifier == 0 {
                break;
            }

            if tag.tag_identifier == 257 {
                if let Ok(fid) = self.parse_file_identifier(&data[pos..]) {
                    let is_dir = (fid.file_characteristics & 0x02) != 0;
                    entries.push(FileEntry {
                        path: fid.file_identifier.clone(),
                        size: 0,
                        is_dir,
                        modified: None,
                        checksum_sha256: None,
                    });
                }
            }

            let next_pos = pos + tag.descriptor_crc_length as usize + 16;
            if next_pos <= pos {
                break;
            }
            pos = next_pos;
        }

        Ok(entries)
    }

    /// Read data from logical block
    fn read_lba(&self, lba: u32) -> Vec<u8> {
        let offset = (lba as usize) * (self.block_size as usize);
        if offset >= self.data.len() {
            return Vec::new();
        }
        let end = std::cmp::min(offset + self.block_size as usize, self.data.len());
        self.data[offset..end].to_vec()
    }
}

impl FilesystemReader for UdfReader {
    fn read_volume(&mut self) -> Result<VolumeInfo, DiskRipperError> {
        let avdp_locations = [
            256u32,
            512,
            (self.data.len() as u32 / 2048).saturating_sub(1),
        ];

        for &lba in &avdp_locations {
            let offset = lba * 2048;
            if (offset as usize) + 16 <= self.data.len() {
                if let Ok(tag) = self.parse_descriptor_tag(&self.data[offset as usize..]) {
                    if tag.tag_identifier == 2 {
                        self.parse_anchor_vdp(offset)?;
                        break;
                    }
                }
            }
        }

        Ok(VolumeInfo {
            volume_id: "UDF Volume".to_string(),
            system_id: "UDF".to_string(),
            volume_size: self.data.len() as u64,
            block_size: self.block_size,
            files_used: 0,
            fs_type: FilesystemType::Udf,
        })
    }

    fn read_directory(&mut self, _path: &str) -> Result<Vec<FileEntry>, DiskRipperError> {
        Ok(self.root_entries.values().cloned().collect())
    }

    fn read_file(
        &mut self,
        _entry: &FileEntry,
        _output_path: &std::path::Path,
    ) -> Result<(), DiskRipperError> {
        Err(DiskRipperError::UnsupportedDisc(
            "UDF file reading not yet fully implemented".to_string(),
        ))
    }

    fn list_files(&mut self) -> Result<Vec<FileEntry>, DiskRipperError> {
        Ok(self.root_entries.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udf_detection() {
        let mut data = vec![0u8; 0x10000];
        data[0x8001..0x8006].copy_from_slice(b"NSR02");
        let reader = UdfReader::new(data);
        assert!(reader.is_ok());
    }

    #[test]
    fn test_udf_detection_nsr03() {
        let mut data = vec![0u8; 0x10000];
        data[0x8001..0x8006].copy_from_slice(b"NSR03");
        let reader = UdfReader::new(data);
        assert!(reader.is_ok());
    }

    #[test]
    fn test_udf_rejection() {
        let data = vec![0u8; 0x10000];
        let reader = UdfReader::new(data);
        assert!(reader.is_err());
    }

    #[test]
    fn test_parse_descriptor_tag() {
        let mut data = vec![0u8; 0x10000];
        data[0x8001..0x8006].copy_from_slice(b"NSR02");
        let reader = UdfReader::new(data).unwrap();

        let mut tag_data = vec![0u8; 16];
        tag_data[0] = 2;
        let tag = reader.parse_descriptor_tag(&tag_data).unwrap();
        assert_eq!(tag.tag_identifier, 2);
    }

    #[test]
    fn test_extent_ad_parsing() {
        let mut data = vec![0u8; 0x10000];
        data[0x8001..0x8006].copy_from_slice(b"NSR02");
        let reader = UdfReader::new(data).unwrap();

        let mut ead = vec![0u8; 8];
        ead[0] = 0x00;
        ead[1] = 0x08;
        ead[4] = 100;

        let (loc, len) = reader.parse_extent_ad(&ead);
        assert_eq!(loc, 100);
        assert_eq!(len, 2048);
    }
}
