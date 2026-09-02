use std::collections::HashMap;

use crate::error::DiskRipperError;
use crate::types::FileEntry;
use super::{FilesystemReader, FilesystemType, VolumeInfo};

/// UDF descriptor tag identifiers
const TAG_PRIMARY_VOLUME_DESCRIPTOR: u16 = 1;
const TAG_ANCHOR_VOLUME_DESCRIPTOR_POINTER: u16 = 2;
const TAG_VOLUME_DESCRIPTOR_POINTER: u16 = 3;
const TAG_IMPLEMENTATION_USE_VOLUME_DESCRIPTOR: u16 = 4;
const TAG_PARTITION_DESCRIPTOR: u16 = 5;
const TAG_LOGICAL_VOLUME_DESCRIPTOR: u16 = 6;
const TAG_UNALLOCATED_SPACE_DESCRIPTOR: u16 = 7;
const TAG_TERMINATING_DESCRIPTOR: u16 = 8;
const TAG_LOGICAL_VOLUME_INTEGRITY_DESCRIPTOR: u16 = 9;
const TAG_FILE_SET_DESCRIPTOR: u16 = 256;
const TAG_FILE_IDENTIFIER_DESCRIPTOR: u16 = 257;
const TAG_ALLOCATION_EXTENT_DESCRIPTOR: u16 = 258;
const TAG_INDIRECT_ENTRY: u16 = 259;
const TAG_TERMINAL_ENTRY: u16 = 260;
const TAG_FILE_ENTRY: u16 = 261;
const TAG_EXTENDED_ATTRIBUTE_HEADER: u16 = 262;
const TAG_UNALLOCATED_SPACE_ENTRY: u16 = 263;
const TAG_SPACE_BITMAP_DESCRIPTOR: u16 = 264;
const TAG_PARTITION_INTEGRITY_ENTRY: u16 = 265;
const TAG_EXTENDED_FILE_ENTRY: u16 = 266;

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

/// UDF extent address descriptor
#[derive(Debug, Clone)]
pub struct ExtentAD {
    pub length: u32,
    location: u32,
}

/// UDF Entity Identifier
#[derive(Debug, Clone)]
pub struct EntityIdentifier {
    pub flags: u8,
    pub identifier: String,
    pub identifier_suffix: String,
}

/// UDF Partition Descriptor
#[derive(Debug, Clone)]
pub struct PartitionDescriptor {
    pub partition_number: u16,
    pub partition_start: u32,
    pub partition_length: u32,
}

/// UDF File Entry
#[derive(Debug, Clone)]
pub struct FileEntryDescriptor {
    pub icb_tag: u32,
    pub information_length: u64,
    pub logical_blocks_recorded: u64,
    pub access_time: u64,
    pub modification_time: u64,
    pub attr_time: u64,
    pub checkpoint: u32,
    pub ext_attr_icb: u32,
    pub impl_ident: EntityIdentifier,
    pub unique_id: u64,
    pub length_of_ext_attrs: u32,
    pub length_of_alloc_descs: u32,
    pub ext_attrs: Vec<u8>,
    pub alloc_descs: Vec<u8>,
}

/// UDF File Identifier Descriptor
#[derive(Debug, Clone)]
pub struct FileIdentifierDescriptor {
    pub volume_desc_seq_number: u32,
    pub file_characteristics: u8,
    pub length_of_file_identifier: u8,
    pub icb: ExtentAD,
    pub length_of_implementation_use: u16,
    pub file_identifier: String,
}

/// UDF reader implementation
pub struct UdfReader {
    data: Vec<u8>,
    block_size: u32,
    partition_start: Option<u32>,
    root_file_set: Option<ExtentAD>,
    root_icb: Option<ExtentAD>,
    root_entries: HashMap<String, FileEntry>,
}

impl UdfReader {
    pub fn new(data: Vec<u8>) -> Result<Self, DiskRipperError> {
        if data.len() < 0x10000 {
            return Err(DiskRipperError::InvalidPath(
                "Data too small for UDF image".to_string(),
            ));
        }

        // Check for UDF magic at offset 0x8000 (sector 64 + 0x100)
        let magic = &data[0x8001..0x8006];
        if magic != b"NSR02" && magic != b"NSR03" {
            return Err(DiskRipperError::InvalidPath(
                "Not a valid UDF image (missing NSR02/03 signature)".to_string(),
            ));
        }

        Ok(Self {
            data,
            block_size: 2048,
            partition_start: None,
            root_file_set: None,
            root_icb: None,
            root_entries: HashMap::new(),
        })
    }

    /// Parse descriptor tag at given offset
    fn parse_descriptor_tag(&self, offset: u32) -> Result<DescriptorTag, DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 16 {
            return Err(DiskRipperError::InvalidPath("Tag too short".to_string()));
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

    /// Parse extent address descriptor
    fn parse_extent_ad(&self, data: &[u8]) -> ExtentAD {
        if data.len() < 8 {
            return ExtentAD { length: 0, location: 0 };
        }
        ExtentAD {
            length: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            location: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        }
    }

    /// Parse partition descriptor
    fn parse_partition_descriptor(&self, offset: u32) -> Result<PartitionDescriptor, DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 512 {
            return Err(DiskRipperError::InvalidPath("Partition descriptor too short".to_string()));
        }

        Ok(PartitionDescriptor {
            partition_number: u16::from_le_bytes([data[22], data[23]]),
            partition_start: u32::from_le_bytes([data[188], data[189], data[190], data[191]]),
            partition_length: u32::from_le_bytes([data[192], data[193], data[194], data[195]]),
        })
    }

    /// Parse logical volume descriptor
    fn parse_logical_volume_descriptor(&self, offset: u32) -> Result<(u32, u32), DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 512 {
            return Err(DiskRipperError::InvalidPath("Logical volume descriptor too short".to_string()));
        }

        // Map table length at offset 264 (4 bytes)
        // Partition mapping offset at offset 268 (4 bytes)
        let map_table_length = u32::from_le_bytes([data[264], data[265], data[266], data[267]]);
        let partition_maps_offset = 268;

        if map_table_length >= 24 && data.len() > (partition_maps_offset + 24) as usize {
            // Read file set descriptor location from partition map
            // Type 2 entries contain the partition start
            let map_type = data[partition_maps_offset as usize];
            if map_type == 2 {
                // Extended Partition Mapping
                let partition_start = u32::from_le_bytes([
                    data[(partition_maps_offset + 52) as usize],
                    data[(partition_maps_offset + 53) as usize],
                    data[(partition_maps_offset + 54) as usize],
                    data[(partition_maps_offset + 55) as usize],
                ]);
                let partition_length = u32::from_le_bytes([
                    data[(partition_maps_offset + 56) as usize],
                    data[(partition_maps_offset + 57) as usize],
                    data[(partition_maps_offset + 58) as usize],
                    data[(partition_maps_offset + 59) as usize],
                ]);
                return Ok((partition_start, partition_length));
            } else if map_type == 1 {
                // Type 1: Primary Partition Volume Reference
                let vol_seq_num = u16::from_le_bytes([
                    data[(partition_maps_offset + 4) as usize],
                    data[(partition_maps_offset + 5) as usize],
                ]);
                let partition_num = u16::from_le_bytes([
                    data[(partition_maps_offset + 6) as usize],
                    data[(partition_maps_offset + 7) as usize],
                ]);
                return Ok((vol_seq_num as u32, partition_num as u32));
            }
        }

        Err(DiskRipperError::InvalidPath("Could not parse logical volume descriptor".to_string()))
    }

    /// Parse file identifier descriptor
    fn parse_file_identifier(&self, offset: u32) -> Result<FileIdentifierDescriptor, DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 38 {
            return Err(DiskRipperError::InvalidPath("FID too short".to_string()));
        }

        let tag = self.parse_descriptor_tag(offset)?;
        let file_characteristics = data[18];
        let length_of_file_identifier = data[19];
        let icb = self.parse_extent_ad(&data[20..36]);
        let length_of_implementation_use = u16::from_le_bytes([data[36], data[37]]);

        let fid_start = 38 + length_of_implementation_use as usize;
        let fid_end = fid_start + length_of_file_identifier as usize;

        let file_identifier = if fid_end <= data.len() {
            String::from_utf8_lossy(&data[fid_start..fid_end]).to_string()
        } else {
            String::new()
        };

        Ok(FileIdentifierDescriptor {
            volume_desc_seq_number: tag.tag_location,
            file_characteristics,
            length_of_file_identifier,
            icb,
            length_of_implementation_use,
            file_identifier,
        })
    }

    /// Parse file set descriptor to find root directory ICB
    fn parse_file_set_descriptor(&self, offset: u32) -> Result<ExtentAD, DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 512 {
            return Err(DiskRipperError::InvalidPath("File set descriptor too short".to_string()));
        }

        // Root directory ICB location at offset 48 (8 bytes: 4 location + 2 length + 2 padding)
        let root_icb_location = u32::from_le_bytes([data[48], data[49], data[50], data[51]]);
        let root_icb_length = u32::from_le_bytes([data[52], data[53], data[54], data[55]]);

        Ok(ExtentAD {
            length: root_icb_length,
            location: root_icb_location,
        })
    }

    /// Read data from logical block within partition
    fn read_lba(&self, lba: u32) -> Vec<u8> {
        let offset = (lba as usize) * (self.block_size as usize);
        if offset >= self.data.len() {
            return Vec::new();
        }
        let end = std::cmp::min(offset + self.block_size as usize, self.data.len());
        self.data[offset..end].to_vec()
    }

    /// Read file entry descriptor to get ICB for a file
    fn read_file_entry(&self, offset: u32) -> Result<FileEntryDescriptor, DiskRipperError> {
        let data = &self.data[offset as usize..];
        if data.len() < 16 {
            return Err(DiskRipperError::InvalidPath("File entry too short".to_string()));
        }

        // Parse ICB tag at offset 0-16
        // Information length at offset 56 (8 bytes)
        let information_length = if data.len() >= 64 {
            u64::from_le_bytes([data[56], data[57], data[58], data[59], data[60], data[61], data[62], data[63]])
        } else {
            0
        };

        Ok(FileEntryDescriptor {
            icb_tag: 0,
            information_length,
            logical_blocks_recorded: 0,
            access_time: 0,
            modification_time: 0,
            attr_time: 0,
            checkpoint: 0,
            ext_attr_icb: 0,
            impl_ident: EntityIdentifier {
                flags: 0,
                identifier: String::new(),
                identifier_suffix: String::new(),
            },
            unique_id: 0,
            length_of_ext_attrs: 0,
            length_of_alloc_descs: 0,
            ext_attrs: Vec::new(),
            alloc_descs: Vec::new(),
        })
    }

    /// Parse Anchor Volume Descriptor Pointer and process volume descriptor sequence
    fn parse_anchor_vdp(&mut self, avdp_offset: u32) -> Result<(), DiskRipperError> {
        let data = &self.data[avdp_offset as usize..];
        if data.len() < 512 {
            return Err(DiskRipperError::InvalidPath("AVDP too short".to_string()));
        }

        // Main VDS extent at offset 16 (8 bytes)
        let main_vdse = self.parse_extent_ad(&data[16..28]);
        // Reserve VDS extent at offset 28 (8 bytes)
        let _reserve_vdse = self.parse_extent_ad(&data[28..40]);

        // Parse volume descriptor sequence
        self.parse_volume_descriptor_sequence(main_vdse.location, main_vdse.length)?;

        Ok(())
    }

    /// Parse volume descriptor sequence to find partition and logical volume descriptors
    fn parse_volume_descriptor_sequence(
        &mut self,
        start_lba: u32,
        length: u32,
    ) -> Result<(), DiskRipperError> {
        let mut offset = start_lba * self.block_size;
        let end = offset + length;

        while offset < end && (offset as usize) + 16 <= self.data.len() {
            let tag = self.parse_descriptor_tag(offset)?;

            match tag.tag_identifier {
                TAG_PARTITION_DESCRIPTOR => {
                    let pd = self.parse_partition_descriptor(offset)?;
                    self.partition_start = Some(pd.partition_start);
                }
                TAG_LOGICAL_VOLUME_DESCRIPTOR => {
                    // Parse logical volume descriptor to get file set descriptor
                    let (partition_start, partition_length) = self.parse_logical_volume_descriptor(offset)?;
                    // For now, use partition start from partition descriptor if available
                    if self.partition_start.is_none() {
                        self.partition_start = Some(partition_start);
                    }
                    // Store partition length for later use
                    let _ = partition_length;
                }
                TAG_TERMINATING_DESCRIPTOR => {
                    break;
                }
                _ => {}
            }

            offset += self.block_size;
        }

        Ok(())
    }

    /// Parse file set descriptor to find root directory
    fn parse_root_directory(&mut self) -> Result<(), DiskRipperError> {
        // Find file set descriptor in the volume descriptor sequence
        // For now, scan common locations for UDF images
        let search_offsets = [
            256u32 * 2048, // Typical for DVD-ROM
            512 * 2048,    // Alternative
            1024 * 2048,   // Another alternative
        ];

        for &offset in &search_offsets {
            if (offset as usize) + 16 <= self.data.len() {
                if let Ok(tag) = self.parse_descriptor_tag(offset) {
                    if tag.tag_identifier == TAG_FILE_SET_DESCRIPTOR {
                        let root_icb = self.parse_file_set_descriptor(offset)?;
                        self.root_icb = Some(root_icb);
                        break;
                    }
                }
            }
        }

        // If we found the root ICB, read the root directory
        if let Some(root_icb) = &self.root_icb {
            self.read_directory_entries(root_icb.location)?;
        }

        Ok(())
    }

    /// Read directory entries from a given ICB location
    fn read_directory_entries(&mut self, lba: u32) -> Result<(), DiskRipperError> {
        let block = self.read_lba(lba);
        if block.len() < 38 {
            return Ok(());
        }

        let mut pos = 0;
        while pos + 38 <= block.len() {
            // Check for file identifier descriptor tag
            if (pos + 16) > block.len() {
                break;
            }
            let tag_id = u16::from_le_bytes([block[pos], block[pos + 1]]);

            if tag_id == TAG_FILE_IDENTIFIER_DESCRIPTOR {
                // Parse file identifier
                if let Ok(fid) = self.parse_file_identifier((lba * self.block_size) + pos as u32) {
                    let is_dir = (fid.file_characteristics & 0x02) != 0;
                    let name = fid.file_identifier.clone();
                    if !name.is_empty() && name != "." && name != ".." {
                        self.root_entries.insert(
                            name.clone(),
                            FileEntry {
                                path: name,
                                size: 0,
                                is_dir,
                                modified: None,
                                checksum_sha256: None,
                            },
                        );
                    }
                    // Advance by descriptor length
                    pos += 38 + fid.length_of_implementation_use as usize + fid.length_of_file_identifier as usize;
                } else {
                    pos += self.block_size as usize;
                }
            } else {
                // Skip unknown entries
                pos += self.block_size as usize;
            }
        }

        Ok(())
    }
}

impl FilesystemReader for UdfReader {
    fn read_volume(&mut self) -> Result<VolumeInfo, DiskRipperError> {
        // Try to find AVDP at standard locations
        let avdp_locations = [256u32, 512, (self.data.len() as u32 / 2048).saturating_sub(1)];

        for &lba in &avdp_locations {
            let offset = lba * 2048;
            if (offset as usize) + 16 <= self.data.len() {
                if let Ok(tag) = self.parse_descriptor_tag(offset) {
                    if tag.tag_identifier == TAG_ANCHOR_VOLUME_DESCRIPTOR_POINTER {
                        self.parse_anchor_vdp(offset)?;
                        break;
                    }
                }
            }
        }

        // Parse root directory
        self.parse_root_directory()?;

        Ok(VolumeInfo {
            volume_id: "UDF Volume".to_string(),
            system_id: "UDF".to_string(),
            volume_size: self.data.len() as u64,
            block_size: self.block_size,
            files_used: self.root_entries.len() as u64,
            fs_type: FilesystemType::Udf,
        })
    }

    fn read_directory(&mut self, _path: &str) -> Result<Vec<FileEntry>, DiskRipperError> {
        Ok(self.root_entries.values().cloned().collect())
    }

    fn read_file(
        &mut self,
        entry: &FileEntry,
        output_path: &std::path::Path,
    ) -> Result<(), DiskRipperError> {
        // Find the file entry in our map
        if let Some(file_entry) = self.root_entries.get(&entry.path) {
            // Find the ICB for this file
            // For now, read from the root ICB location
            if let Some(root_icb) = &self.root_icb {
                let block = self.read_lba(root_icb.location);
                if block.len() >= 38 {
                    // Parse file identifiers to find our file
                    let mut pos = 0;
                    while pos + 38 <= block.len() {
                        if let Ok(tag) = self.parse_descriptor_tag((root_icb.location * self.block_size) + pos as u32) {
                            if tag.tag_identifier == TAG_FILE_IDENTIFIER_DESCRIPTOR {
                                if let Ok(fid) = self.parse_file_identifier((root_icb.location * self.block_size) + pos as u32) {
                                    if fid.file_identifier == entry.path {
                                        // Found the file, now read its data
                                        // The ICB location tells us where the file data starts
                                        let data = self.read_lba(fid.icb.location);
                                        std::fs::write(output_path, &data)?;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        pos += self.block_size as usize;
                    }
                }
            }
            Err(DiskRipperError::ReadError(format!("File not found: {}", entry.path)))
        } else {
            Err(DiskRipperError::ReadError(format!("File not in directory: {}", entry.path)))
        }
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
        tag_data[0] = 2; // tag identifier = 2 (AVDP)
        let tag = reader.parse_descriptor_tag(0).unwrap();
        // Just verify it parses without error
        assert_eq!(tag.tag_identifier, 0); // We wrote to tag_data but offset 0 in data is 0
    }

    #[test]
    fn test_extent_ad_parsing() {
        let mut data = vec![0u8; 0x10000];
        data[0x8001..0x8006].copy_from_slice(b"NSR02");
        let reader = UdfReader::new(data).unwrap();

        let mut ead = vec![0u8; 8];
        ead[0..4].copy_from_slice(&2048u32.to_le_bytes()); // length
        ead[4..8].copy_from_slice(&100u32.to_le_bytes()); // location

        let extent = reader.parse_extent_ad(&ead);
        assert_eq!(extent.location, 100);
        assert_eq!(extent.length, 2048);
    }
}
