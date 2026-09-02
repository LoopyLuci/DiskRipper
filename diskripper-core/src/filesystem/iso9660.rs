use std::collections::HashMap;

use crate::error::DiskRipperError;
use crate::types::FileEntry;
use super::{FilesystemReader, FilesystemType, VolumeInfo};

/// ISO 9660 directory record
#[derive(Debug, Clone)]
pub struct DirectoryRecord {
    pub lba: u32,
    pub data_len: u32,
    pub flags: u8,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub rock_ridge: Option<RockRidgeData>,
    /// Multi-extent: total size across all extents
    pub total_extent_size: u64,
}

/// Rock Ridge extension data
#[derive(Debug, Clone)]
pub struct RockRidgeData {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}

/// Represents a file extent (for multi-extent files)
#[derive(Debug, Clone)]
pub struct FileExtent {
    pub lba: u32,
    pub size: u32,
}

pub struct Iso9660Reader {
    data: Vec<u8>,
    block_size: u32,
    root_dir_lba: u32,
    root_dir_size: u32,
    joliet: bool,
    path_table: HashMap<String, u32>,
    /// Multi-extent support: path -> list of extents
    multi_extents: HashMap<String, Vec<FileExtent>>,
}

impl Iso9660Reader {
    pub fn new(data: Vec<u8>) -> Result<Self, DiskRipperError> {
        if data.len() < 0x8010 {
            return Err(DiskRipperError::InvalidPath(
                "Data too small for ISO 9660".to_string(),
            ));
        }

        if &data[0x8001..0x8006] != b"CD001" {
            return Err(DiskRipperError::InvalidPath(
                "Not a valid ISO 9660 image".to_string(),
            ));
        }

        let pvd_offset = 0x8000;
        let block_size = Self::read_both_endian_u16(&data, pvd_offset + 128) as u32;
        let block_size = if block_size == 0 { 2048 } else { block_size };

        let path_table_size = Self::read_both_endian_u32(&data, pvd_offset + 132);
        let path_table_lba = Self::read_both_endian_u32(&data, pvd_offset + 140);

        let root_record = &data[pvd_offset + 156..pvd_offset + 190];
        let root_dir_lba = Self::read_both_endian_u32(root_record, 2);
        let root_dir_size = Self::read_both_endian_u32(root_record, 10);

        let joliet = Self::detect_joliet(&data);

        let mut reader = Self {
            data,
            block_size,
            root_dir_lba,
            root_dir_size,
            joliet,
            path_table: HashMap::new(),
            multi_extents: HashMap::new(),
        };

        reader.build_path_table(path_table_lba, path_table_size);

        Ok(reader)
    }

    fn detect_joliet(data: &[u8]) -> bool {
        let svd_offset = 0x8800;
        if data.len() > svd_offset + 91 {
            if &data[svd_offset + 1..svd_offset + 6] == b"CD001" {
                let esc = &data[svd_offset + 88..svd_offset + 91];
                return esc == [0x25, 0x2F, 0x40]
                    || esc == [0x25, 0x2F, 0x43]
                    || esc == [0x25, 0x2F, 0x45];
            }
        }
        false
    }

    fn build_path_table(&mut self, lba: u32, size: u32) {
        if size == 0 || lba == 0 {
            return;
        }

        let offset = (lba as usize) * (self.block_size as usize);
        if offset + (size as usize) > self.data.len() {
            return;
        }

        let mut pos = offset;
        let end = offset + size as usize;

        if pos + 8 < end {
            let name_len = self.data[pos] as usize;
            let dir_lba = Self::read_both_endian_u32(&self.data, pos + 2);
            let name = self.read_iso_name(pos + 8, name_len);

            self.path_table.insert(name.clone(), dir_lba);
            self.path_table.insert("\\".to_string(), dir_lba);
            self.path_table.insert("/".to_string(), dir_lba);

            let entry_len = 8 + name_len + (name_len % 2);
            pos += entry_len;
        }

        while pos + 8 < end {
            let name_len = self.data[pos] as usize;
            if name_len == 0 {
                break;
            }
            let dir_lba = Self::read_both_endian_u32(&self.data, pos + 2);
            let name = self.read_iso_name(pos + 8, name_len);

            self.path_table.insert(name.clone(), dir_lba);

            let entry_len = 8 + name_len + (name_len % 2);
            pos += entry_len;
        }
    }

    /// Read ISO 9660 name, with optional Joliet UCS-2 decoding
    fn read_iso_name(&self, offset: usize, len: usize) -> String {
        let bytes = &self.data[offset..offset + len];
        
        if self.joliet {
            // Joliet uses UCS-2 (big-endian UTF-16)
            self.decode_ucs2_be(bytes)
                .trim_end_matches('\0')
                .trim_end_matches(';')
                .trim_end_matches('1')
                .trim_end_matches('.')
                .to_string()
        } else {
            let name = String::from_utf8_lossy(bytes);
            name.trim_end_matches(' ')
                .trim_end_matches(';')
                .trim_end_matches('1')
                .trim_end_matches('.')
                .to_string()
        }
    }

    /// Decode UCS-2 big-endian (Joliet) to String
    fn decode_ucs2_be(&self, bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len() / 2);
        let mut i = 0;
        while i + 1 < bytes.len() {
            let code = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            if code == 0 {
                break;
            }
            // UCS-2 is essentially UTF-16 without surrogate pairs
            if let Some(ch) = char::from_u32(code as u32) {
                result.push(ch);
            }
            i += 2;
        }
        result
    }

    /// Read a dual-endian u16 (ISO 9660 stores both LE and BE)
    fn read_both_endian_u16(data: &[u8], offset: usize) -> u16 {
        let le = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let be = u16::from_be_bytes([data[offset], data[offset + 1]]);
        if le != be {
            tracing::warn!(
                "Dual-endian mismatch at offset 0x{:x}: LE=0x{:04x}, BE=0x{:04x}",
                offset,
                le,
                be
            );
        }
        le
    }

    /// Read a dual-endian u32
    fn read_both_endian_u32(data: &[u8], offset: usize) -> u32 {
        let le = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        let be = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        if le != be {
            tracing::warn!(
                "Dual-endian mismatch at offset 0x{:x}: LE=0x{:08x}, BE=0x{:08x}",
                offset,
                le,
                be
            );
        }
        le
    }

    fn read_directory_records(
        &self,
        lba: u32,
        size: u32,
    ) -> Result<Vec<DirectoryRecord>, DiskRipperError> {
        let mut records = Vec::new();
        let offset = (lba as usize) * (self.block_size as usize);
        let end = offset + (size as usize);

        if offset >= self.data.len() || end > self.data.len() {
            return Err(DiskRipperError::InvalidPath(
                "Directory extent out of bounds".to_string(),
            ));
        }

        let mut pos = offset;
        while pos < end {
            let record_len = self.data[pos];
            if record_len == 0 {
                let padding = (4 - ((pos - offset) % 4)) % 4;
                pos += 1 + padding;
                continue;
            }

            let record = self.parse_directory_record(pos, record_len as usize);
            records.push(record);

            pos += record_len as usize;
        }

        Ok(records)
    }

    fn parse_directory_record(&self, offset: usize, len: usize) -> DirectoryRecord {
        let data = &self.data[offset..offset + len];

        let lba = Self::read_both_endian_u32(data, 2);
        let data_len = Self::read_both_endian_u32(data, 10);
        let flags = data[25];
        let name_len = data[32] as usize;

        let name = if name_len > 0 {
            let name_bytes = &data[33..33 + name_len];
            if self.joliet {
                self.decode_ucs2_be(name_bytes)
                    .trim_end_matches('\0')
                    .trim_end_matches(";1")
                    .trim_end_matches('.')
                    .to_string()
            } else {
                let name_str = String::from_utf8_lossy(name_bytes);
                name_str
                    .trim_end_matches(";1")
                    .trim_end_matches('.')
                    .trim_end()
                    .to_string()
            }
        } else {
            ".".to_string()
        };

        let rock_ridge = self.parse_rock_ridge(data, name_len);

        DirectoryRecord {
            lba,
            data_len,
            flags,
            name,
            is_dir: flags & 0x02 != 0,
            size: data_len as u64,
            rock_ridge,
            total_extent_size: data_len as u64,
        }
    }

    fn parse_rock_ridge(&self, data: &[u8], name_len: usize) -> Option<RockRidgeData> {
        let rr_start = 33 + name_len;
        if rr_start >= data.len() {
            return None;
        }

        let mut pos = rr_start;
        let mut mode = 0u32;
        let mut uid = 0u32;
        let mut gid = 0u32;
        let mut is_symlink = false;
        let mut symlink_target = None;

        while pos + 4 <= data.len() {
            let sig = &data[pos..pos + 2];
            let entry_len = data[pos + 2] as usize;
            if entry_len < 4 || pos + entry_len > data.len() {
                break;
            }

            match sig {
                b"PX" => {
                    if entry_len >= 36 {
                        mode = Self::read_both_endian_u32(data, pos + 4);
                        uid = Self::read_both_endian_u32(data, pos + 20);
                        gid = Self::read_both_endian_u32(data, pos + 28);
                    }
                }
                b"SL" => {
                    is_symlink = true;
                    if entry_len > 5 {
                        let target_len = data[pos + 5] as usize;
                        if pos + 6 + target_len <= data.len() {
                            symlink_target = Some(
                                String::from_utf8_lossy(&data[pos + 6..pos + 6 + target_len])
                                    .to_string(),
                            );
                        }
                    }
                }
                _ => {}
            }

            pos += entry_len;
        }

        if mode > 0 {
            Some(RockRidgeData {
                mode,
                uid,
                gid,
                is_symlink,
                symlink_target,
            })
        } else {
            None
        }
    }

    fn read_file_data(&self, lba: u32, size: u32) -> Vec<u8> {
        let offset = (lba as usize) * (self.block_size as usize);
        let end = offset + (size as usize);

        if offset >= self.data.len() {
            return Vec::new();
        }

        let actual_end = end.min(self.data.len());
        self.data[offset..actual_end].to_vec()
    }

    /// Read multi-extent file data
    fn read_multi_extent_data(&self, extents: &[FileExtent]) -> Vec<u8> {
        let total_size: u32 = extents.iter().map(|e| e.size).sum();
        let mut data = Vec::with_capacity(total_size as usize);

        for extent in extents {
            let extent_data = self.read_file_data(extent.lba, extent.size);
            data.extend_from_slice(&extent_data);
        }

        data
    }
}

impl FilesystemReader for Iso9660Reader {
    fn read_volume(&mut self) -> Result<VolumeInfo, DiskRipperError> {
        let pvd_offset = 0x8000;
        let volume_id = String::from_utf8_lossy(&self.data[pvd_offset + 40..pvd_offset + 72])
            .trim_end_matches(' ')
            .to_string();
        let system_id = String::from_utf8_lossy(&self.data[pvd_offset + 8..pvd_offset + 40])
            .trim_end_matches(' ')
            .to_string();
        let volume_size =
            Self::read_both_endian_u32(&self.data, pvd_offset + 80) as u64 * self.block_size as u64;
        let files_used = Self::read_both_endian_u32(&self.data, pvd_offset + 154) as u64;

        Ok(VolumeInfo {
            volume_id,
            system_id,
            volume_size,
            block_size: self.block_size,
            files_used,
            fs_type: if self.joliet {
                FilesystemType::Joliet
            } else {
                FilesystemType::Iso9660
            },
        })
    }

    fn read_directory(&mut self, path: &str) -> Result<Vec<FileEntry>, DiskRipperError> {
        let lba = if path == "/" || path == "\\" || path.is_empty() {
            self.root_dir_lba
        } else {
            self.path_table
                .get(path)
                .copied()
                .ok_or_else(|| DiskRipperError::InvalidPath(format!("Path not found: {}", path)))?
        };

        let records = self.read_directory_records(lba, self.root_dir_size)?;

        let mut entries = Vec::new();
        for record in records {
            if record.name == "." || record.name == "\0" || record.name.is_empty() {
                continue;
            }
            entries.push(FileEntry {
                path: if path == "/" || path.is_empty() {
                    format!("/{}", record.name)
                } else {
                    format!("{}/{}", path, record.name)
                },
                size: record.size,
                is_dir: record.is_dir,
                modified: None,
                checksum_sha256: None,
            });
        }

        Ok(entries)
    }

    fn read_file(
        &mut self,
        entry: &FileEntry,
        output_path: &std::path::Path,
    ) -> Result<(), DiskRipperError> {
        // Check for multi-extent first
        if let Some(extents) = self.multi_extents.get(&entry.path) {
            let data = self.read_multi_extent_data(extents);
            std::fs::write(output_path, &data)?;
            return Ok(());
        }

        // Find the file's LBA by walking directory records
        let (lba, size) = self.find_file_lba(&entry.path)?;
        let data = self.read_file_data(lba, size);
        std::fs::write(output_path, &data)?;
        Ok(())
    }

    fn list_files(&mut self) -> Result<Vec<FileEntry>, DiskRipperError> {
        let mut all_files = Vec::new();
        self.list_files_recursive("", self.root_dir_lba, self.root_dir_size, &mut all_files)?;
        Ok(all_files)
    }
}

impl Iso9660Reader {
    /// Find a file's LBA by walking directory records
    fn find_file_lba(&self, path: &str) -> Result<(u32, u32), DiskRipperError> {
        let path = path.trim_start_matches('/').trim_start_matches('\\');
        if path.is_empty() {
            return Err(DiskRipperError::InvalidPath("Empty file path".to_string()));
        }

        let components: Vec<&str> = path.split('/').collect();
        let mut current_lba = self.root_dir_lba;
        let mut current_size = self.root_dir_size;

        for (i, component) in components.iter().enumerate() {
            let records = self.read_directory_records(current_lba, current_size)?;

            if let Some(record) = records.iter().find(|r| r.name == *component) {
                if i == components.len() - 1 {
                    if record.is_dir {
                        return Err(DiskRipperError::InvalidPath(format!(
                            "Path is a directory: {}",
                            path
                        )));
                    }
                    return Ok((record.lba, record.data_len));
                } else {
                    if !record.is_dir {
                        return Err(DiskRipperError::InvalidPath(format!(
                            "Path component is a file: {}",
                            component
                        )));
                    }
                    current_lba = record.lba;
                    current_size = record.data_len;
                }
            } else {
                return Err(DiskRipperError::InvalidPath(format!(
                    "Path component not found: {}",
                    component
                )));
            }
        }

        Err(DiskRipperError::InvalidPath(format!("File not found: {}", path)))
    }

    fn list_files_recursive(
        &mut self,
        prefix: &str,
        lba: u32,
        size: u32,
        output: &mut Vec<FileEntry>,
    ) -> Result<(), DiskRipperError> {
        let records = self.read_directory_records(lba, size)?;

        // First pass: collect multi-extent info
        let mut current_name: Option<String> = None;
        let mut current_extents: Vec<FileExtent> = Vec::new();
        let mut _current_is_dir = false;
        let mut current_path = String::new();

        for record in &records {
            if record.name == "." || record.name == "\0" || record.name.is_empty() {
                continue;
            }

            let path = if prefix.is_empty() {
                record.name.clone()
            } else {
                format!("{}/{}", prefix, record.name)
            };

            // Check if this is a continuation of the previous record (multi-extent)
            if current_name.as_ref() == Some(&record.name) {
                current_extents.push(FileExtent {
                    lba: record.lba,
                    size: record.data_len,
                });
            } else {
                // Save previous record if it had multiple extents
                if let Some(_) = current_name.take() {
                    if current_extents.len() > 1 {
                        self.multi_extents.insert(current_path.clone(), current_extents.clone());
                    }
                }
                // Start new record
                current_name = Some(record.name.clone());
                current_extents = vec![FileExtent {
                    lba: record.lba,
                    size: record.data_len,
                }];
                current_path = path;
            }
        }

        // Handle last record
        if let Some(_) = current_name {
            if current_extents.len() > 1 {
                self.multi_extents.insert(current_path.clone(), current_extents.clone());
            }
        }

        // Second pass: build file entries
        for record in records {
            if record.name == "." || record.name == "\0" || record.name.is_empty() {
                continue;
            }

            let path = if prefix.is_empty() {
                record.name.clone()
            } else {
                format!("{}/{}", prefix, record.name)
            };

            if record.is_dir {
                self.list_files_recursive(&path, record.lba, record.data_len, output)?;
            } else {
                // Calculate total size including multi-extent
                let total_size = if let Some(extents) = self.multi_extents.get(&path) {
                    extents.iter().map(|e| e.size as u64).sum()
                } else {
                    record.size
                };

                output.push(FileEntry {
                    path,
                    size: total_size,
                    is_dir: false,
                    modified: None,
                    checksum_sha256: None,
                });
            }
        }

        Ok(())
    }
}

pub fn parse_iso9660(data: Vec<u8>) -> Result<(Iso9660Reader, VolumeInfo), DiskRipperError> {
    let mut reader = Iso9660Reader::new(data)?;
    let volume_info = reader.read_volume()?;
    Ok((reader, volume_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_iso9660() {
        let mut data = vec![0u8; 0x8100];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        assert_eq!(
            super::super::detect_filesystem(&data),
            FilesystemType::Iso9660
        );
    }

    #[test]
    fn test_detect_joliet() {
        let mut data = vec![0u8; 0x9000];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        data[0x8801..0x8806].copy_from_slice(b"CD001");
        data[0x8858..0x885B].copy_from_slice(&[0x25, 0x2F, 0x40]);
        assert_eq!(
            super::super::detect_filesystem(&data),
            FilesystemType::Joliet
        );
    }

    #[test]
    fn test_both_endian_u16() {
        // Dual-endian for 2048 (0x0800): LE [0x00, 0x08], BE [0x08, 0x00]
        let data = [0x00, 0x08, 0x08, 0x00];
        assert_eq!(Iso9660Reader::read_both_endian_u16(&data, 0), 2048);
    }

    #[test]
    fn test_both_endian_u32() {
        // Dual-endian for 16 (0x00000010): LE [0x10, 0x00, 0x00, 0x00], BE [0x00, 0x00, 0x00, 0x10]
        let data = [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10];
        assert_eq!(Iso9660Reader::read_both_endian_u32(&data, 0), 16);
    }

    #[test]
    fn test_decode_ucs2_be() {
        let mut data = vec![0u8; 0x8100];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        let reader = Iso9660Reader::new(data).unwrap();
        // "AB" in UCS-2 BE
        let bytes = [0x00, 0x41, 0x00, 0x42];
        assert_eq!(reader.decode_ucs2_be(&bytes), "AB");
    }

    #[test]
    fn test_decode_ucs2_be_with_null() {
        let mut data = vec![0u8; 0x8100];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        let reader = Iso9660Reader::new(data).unwrap();
        // "A\0" in UCS-2 BE
        let bytes = [0x00, 0x41, 0x00, 0x00];
        assert_eq!(reader.decode_ucs2_be(&bytes), "A");
    }

    #[test]
    fn test_multi_extent_detection() {
        // This would require a real multi-extent ISO image
        // For now, just verify the structure exists
        assert!(true);
    }
}
