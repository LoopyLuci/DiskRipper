//! Professional application icons module.
//!
//! Generates and manages application icons in all required formats:
//! - Windows: ICO (16x16, 32x32, 48x48, 256x256)
//! - macOS: ICNS (16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024)
//! - Linux: PNG (16x16, 32x32, 48x48, 128x128, 256x256, 512x512)

use std::path::Path;

use tracing::info;

use crate::error::DiskRipperError;

/// Icon sizes for different platforms
pub const ICON_SIZES: &[u32] = &[16, 32, 48, 128, 256, 512];

/// Icon manager
pub struct IconManager;

impl IconManager {
    /// Generate all required icon formats
    ///
    /// Creates icons in all sizes and formats for the current platform.
    pub fn generate_icons(output_dir: &Path) -> Result<(), DiskRipperError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;

        info!("Generating application icons...");

        // Generate PNG icons for all sizes
        for &size in ICON_SIZES {
            let png_path = output_dir.join(format!("icon_{}x{}.png", size, size));
            Self::generate_png_icon(&png_path, size)?;
        }

        // Generate platform-specific formats
        #[cfg(target_os = "windows")]
        {
            Self::generate_ico(output_dir)?;
        }

        #[cfg(target_os = "macos")]
        {
            Self::generate_icns(output_dir)?;
        }

        info!("Icon generation complete");
        Ok(())
    }

    /// Generate a PNG icon at the specified size
    ///
    /// Creates a simple but professional-looking icon:
    /// - Blue gradient background
    /// - White disc shape in center
    /// - Transparent background option
    fn generate_png_icon(path: &Path, size: u32) -> Result<(), DiskRipperError> {
        use std::io::Write;

        // Create a simple BMP-like structure and convert to PNG
        // For now, we'll create a minimal valid PNG
        
        let width = size;
        let height = size;
        let mut png_data = Vec::new();

        // PNG signature
        png_data.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

        // IHDR chunk
        let ihdr_data = Self::create_ihdr(width, height);
        Self::write_png_chunk(&mut png_data, b"IHDR", &ihdr_data);

        // IDAT chunk (image data)
        let image_data = Self::create_image_data(width, height);
        let compressed = Self::compress_data(&image_data);
        Self::write_png_chunk(&mut png_data, b"IDAT", &compressed);

        // IEND chunk
        Self::write_png_chunk(&mut png_data, b"IEND", &[]);

        std::fs::write(path, png_data)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;

        Ok(())
    }

    /// Create IHDR chunk data
    fn create_ihdr(width: u32, height: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.push(8); // Bit depth
        data.push(2); // Color type: RGB
        data.push(0); // Compression method
        data.push(0); // Filter method
        data.push(0); // Interlace method
        data
    }

    /// Create RGB image data for the icon
    fn create_image_data(width: u32, height: u32) -> Vec<u8> {
        let mut data = Vec::new();
        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;
        let radius = (width.min(height) as f64 / 2.0) * 0.8;

        for y in 0..height {
            data.push(0); // Filter type: None
            for x in 0..width {
                let dx = x as f64 - center_x;
                let dy = y as f64 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius {
                    // Inside the disc - blue gradient
                    let intensity = 1.0 - (dist / radius) * 0.3;
                    let r = (30.0 * intensity) as u8;
                    let g = (100.0 * intensity) as u8;
                    let b = (200.0 * intensity) as u8;
                    data.push(r);
                    data.push(g);
                    data.push(b);
                } else {
                    // Outside - transparent (white for now)
                    data.push(255);
                    data.push(255);
                    data.push(255);
                }
            }
        }

        data
    }

    /// Simple zlib compression placeholder
    fn compress_data(data: &[u8]) -> Vec<u8> {
        // In production, use flate2 or similar
        // For now, return uncompressed data
        data.to_vec()
    }

    /// Write a PNG chunk with CRC
    fn write_png_chunk(data: &mut Vec<u8>, chunk_type: &[u8], chunk_data: &[u8]) {
        let length = chunk_data.len() as u32;
        data.extend_from_slice(&length.to_be_bytes());
        data.extend_from_slice(chunk_type);
        data.extend_from_slice(chunk_data);

        // Calculate CRC32
        let mut crc = crc32fast::Hasher::new();
        crc.update(chunk_type);
        crc.update(chunk_data);
        let crc_value = crc.finalize();
        data.extend_from_slice(&crc_value.to_be_bytes());
    }

    /// Generate Windows ICO file
    #[cfg(target_os = "windows")]
    fn generate_ico(output_dir: &Path) -> Result<(), DiskRipperError> {
        use std::io::Write;

        let ico_path = output_dir.join("icon.ico");
        let mut ico_data = Vec::new();

        // ICO header
        ico_data.extend_from_slice(&0u16.to_le_bytes()); // Reserved
        ico_data.extend_from_slice(&1u16.to_le_bytes()); // Type: ICO
        ico_data.extend_from_slice(&6u16.to_le_bytes()); // Number of images

        // Image directory entries
        let sizes = [16u32, 32, 48, 128, 256];
        let mut image_data = Vec::new();

        for &size in &sizes {
            // Directory entry
            ico_data.push(size as u8); // Width
            ico_data.push(size as u8); // Height
            ico_data.push(0); // Color palette
            ico_data.push(0); // Reserved
            ico_data.extend_from_slice(&1u16.to_le_bytes()); // Color planes
            ico_data.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel
            let data_offset = 6 + 16 * 6 + image_data.len() as u32;
            ico_data.extend_from_slice(&data_offset.to_le_bytes()); // Offset
            ico_data.extend_from_slice(&0u32.to_le_bytes()); // Size (placeholder)

            // Generate PNG data for this size
            let png = Self::create_png_bytes(size, size)?;
            let entry_offset = 6 + 16 * (sizes.iter().position(|&s| s == size).unwrap() + 1);
            let png_len = png.len() as u32;
            ico_data[entry_offset - 4..entry_offset].copy_from_slice(&png_len.to_le_bytes());
            image_data.extend_from_slice(&png);
        }

        ico_data.extend_from_slice(&image_data);

        std::fs::write(&ico_path, ico_data)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;

        info!("Generated Windows ICO: {}", ico_path.display());
        Ok(())
    }

    /// Generate macOS ICNS file
    #[cfg(target_os = "macos")]
    fn generate_icns(output_dir: &Path) -> Result<(), DiskRipperError> {
        let icns_path = output_dir.join("icon.icns");
        let mut icns_data = Vec::new();

        // ICNS header
        icns_data.extend_from_slice(b"icns"); // Magic
        icns_data.extend_from_slice(&0u32.to_be_bytes()); // File length (placeholder)

        let icns_magic_offset = 4;
        let mut total_length = 8; // Header size

        // Icon types and their sizes
        let icon_types = [
            (0x49434E23, 16),   // ICN# - 16x16
            (0x69637334, 16),   // ics4 - 16x16
            (0x69637338, 32),   // ics8 - 32x32
            (0x69636C34, 128),  // icl4 - 128x128
            (0x69636C38, 128),  // icl8 - 128x128
            (0x69683332, 256),  // ih32 - 256x256
            (0x696C3332, 512),  // il32 - 512x512
            (0x69743332, 1024), // it32 - 1024x1024
        ];

        for (icon_type, size) in &icon_types {
            let png_data = Self::create_png_bytes(*size, *size)?;
            let chunk_length = png_data.len() as u32 + 8;

            icns_data.extend_from_slice(&icon_type.to_be_bytes());
            icns_data.extend_from_slice(&chunk_length.to_be_bytes());
            icns_data.extend_from_slice(&png_data);

            total_length += chunk_length;
        }

        // Update file length
        icns_data[icns_magic_offset..icns_magic_offset + 4]
            .copy_from_slice(&total_length.to_be_bytes());

        std::fs::write(&icns_path, icns_data)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;

        info!("Generated macOS ICNS: {}", icns_path.display());
        Ok(())
    }

    /// Create PNG bytes for a given size
    fn create_png_bytes(width: u32, height: u32) -> Result<Vec<u8>, DiskRipperError> {
        let mut png_data = Vec::new();

        // PNG signature
        png_data.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

        // IHDR
        let ihdr_data = Self::create_ihdr(width, height);
        Self::write_png_chunk(&mut png_data, b"IHDR", &ihdr_data);

        // IDAT
        let image_data = Self::create_image_data(width, height);
        let compressed = Self::compress_data(&image_data);
        Self::write_png_chunk(&mut png_data, b"IDAT", &compressed);

        // IEND
        Self::write_png_chunk(&mut png_data, b"IEND", &[]);

        Ok(png_data)
    }
}

/// Generate placeholder icons for development
///
/// Creates simple placeholder icons when the full icon generator is not available.
pub fn generate_placeholder_icons(output_dir: &Path) -> Result<(), DiskRipperError> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| DiskRipperError::Io(e.to_string()))?;

    // Create a minimal valid PNG (1x1 pixel, blue)
    let png_data: Vec<u8> = vec![
        137, 80, 78, 71, 13, 10, 26, 10, // PNG signature
        0, 0, 0, 13, // IHDR length
        73, 72, 68, 82, // IHDR
        0, 0, 0, 1, // Width: 1
        0, 0, 0, 1, // Height: 1
        8, // Bit depth: 8
        2, // Color type: RGB
        0, 0, 0, // Compression, filter, interlace
        0x3A, 0x24, 0x91, 0x27, // CRC32
        0, 0, 0, 11, // IDAT length
        73, 68, 65, 84, // IDAT
        0x78, 0x9C, 0x62, 0x64, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // Compressed data
        0x1D, 0xB4, 0x63, 0x07, // CRC32
        0, 0, 0, 0, // IEND length
        73, 69, 78, 68, // IEND
        0xAE, 0x42, 0x60, 0x82, // CRC32
    ];

    // Write icons in all required sizes
    for &size in &[32, 128, 256] {
        let path = output_dir.join(format!("icon_{}x{}.png", size, size));
        std::fs::write(&path, &png_data)
            .map_err(|e| DiskRipperError::Io(e.to_string()))?;
    }

    // Write main icon
    std::fs::write(output_dir.join("icon.png"), &png_data)
        .map_err(|e| DiskRipperError::Io(e.to_string()))?;

    info!("Generated placeholder icons in {}", output_dir.display());
    Ok(())
}
