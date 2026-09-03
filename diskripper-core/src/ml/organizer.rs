//! Smart content organization using ML.
//!
//! Automatically organizes ripped content:
//! - Music: Artist/Album/Track structure
//! - Movies: Title/Year/Quality structure
//! - TV Shows: Title/Season/Episode structure
//! - Software: Name/Version structure

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::DiskRipperError;
use crate::ml::pipeline::PipelineResult;

/// Smart organizer for ripped content
pub struct SmartOrganizer {
    output_dir: std::path::PathBuf,
    /// Whether to rename files based on ML identification
    rename_files: bool,
    /// Whether to create NFO files for media centers
    create_nfo: bool,
    /// Whether to download artwork
    download_artwork: bool,
}

/// Organization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResult {
    pub original_path: String,
    pub organized_path: String,
    pub content_type: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub quality: Option<String>,
}

impl SmartOrganizer {
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
            rename_files: true,
            create_nfo: true,
            download_artwork: false,
        }
    }

    /// Organize a ripped file based on ML identification
    pub fn organize(
        &self,
        file_path: &Path,
        identification: &PipelineResult,
    ) -> Result<OrganizationResult, DiskRipperError> {
        let organized_path = self.build_path(file_path, identification)?;

        // Create directory structure
        if let Some(parent) = organized_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DiskRipperError::Io(format!("Failed to create directory: {}", e)))?;
        }

        // Move/rename file
        if self.rename_files && file_path != organized_path {
            std::fs::rename(file_path, &organized_path)
                .map_err(|e| DiskRipperError::Io(format!("Failed to move file: {}", e)))?;
        }

        // Create NFO file
        if self.create_nfo {
            self.create_nfo_file(&organized_path, identification)?;
        }

        Ok(OrganizationResult {
            original_path: file_path.to_string_lossy().to_string(),
            organized_path: organized_path.to_string_lossy().to_string(),
            content_type: format!("{:?}", identification.content_type),
            title: identification.title.clone().unwrap_or_default(),
            artist: identification.artist.clone(),
            album: identification.album.clone(),
            year: identification.year,
            quality: None,
        })
    }

    /// Build organized file path based on identification
    fn build_path(
        &self,
        original_path: &Path,
        identification: &PipelineResult,
    ) -> Result<std::path::PathBuf, DiskRipperError> {
        let ext = original_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let path = match identification.content_type {
            crate::ml::pipeline::ContentType::Music => {
                let artist = identification.artist.as_deref().unwrap_or("Unknown Artist");
                let album = identification.album.as_deref().unwrap_or("Unknown Album");
                let title = identification.title.as_deref().unwrap_or("Unknown Track");

                // Sanitize filenames
                let artist = sanitize_filename(artist);
                let album = sanitize_filename(album);
                let title = sanitize_filename(title);

                self.output_dir
                    .join("Music")
                    .join(&artist)
                    .join(&album)
                    .join(format!("{}.{}", title, ext))
            }
            crate::ml::pipeline::ContentType::Movie => {
                let title = identification.title.as_deref().unwrap_or("Unknown Movie");
                let year = identification.year.map(|y| y.to_string()).unwrap_or_default();

                let title = sanitize_filename(title);

                self.output_dir
                    .join("Movies")
                    .join(format!("{} ({})", title, year))
                    .join(format!("{}.{}", title, ext))
            }
            crate::ml::pipeline::ContentType::TvShow => {
                let title = identification.title.as_deref().unwrap_or("Unknown Show");
                let title = sanitize_filename(title);

                self.output_dir
                    .join("TV Shows")
                    .join(&title)
                    .join(format!("{}.{}", title, ext))
            }
            crate::ml::pipeline::ContentType::Software => {
                let title = identification.title.as_deref().unwrap_or("Unknown Software");
                let title = sanitize_filename(title);

                self.output_dir
                    .join("Software")
                    .join(&title)
                    .join(format!("{}.{}", title, ext))
            }
            crate::ml::pipeline::ContentType::Game => {
                let title = identification.title.as_deref().unwrap_or("Unknown Game");
                let title = sanitize_filename(title);

                self.output_dir
                    .join("Games")
                    .join(&title)
                    .join(format!("{}.{}", title, ext))
            }
            _ => {
                let title = identification.title.as_deref().unwrap_or("Unknown");
                let title = sanitize_filename(title);

                self.output_dir.join("Other").join(format!("{}.{}", title, ext))
            }
        };

        Ok(path)
    }

    /// Create NFO file for media center compatibility
    fn create_nfo_file(
        &self,
        media_path: &Path,
        identification: &PipelineResult,
    ) -> Result<(), DiskRipperError> {
        let nfo_path = media_path.with_extension("nfo");

        let nfo_content = match identification.content_type {
            crate::ml::pipeline::ContentType::Music => {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                    <album>\n\
                    \t<title>{}</title>\n\
                    \t<artist>{}</artist>\n\
                    \t<year>{}</year>\n\
                    \t<genre>{}</genre>\n\
                    </album>",
                    identification.title.as_deref().unwrap_or(""),
                    identification.artist.as_deref().unwrap_or(""),
                    identification.year.unwrap_or(0),
                    identification.genre.as_deref().unwrap_or(""),
                )
            }
            crate::ml::pipeline::ContentType::Movie => {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                    <movie>\n\
                    \t<title>{}</title>\n\
                    \t<year>{}</year>\n\
                    \t<genre>{}</genre>\n\
                    </movie>",
                    identification.title.as_deref().unwrap_or(""),
                    identification.year.unwrap_or(0),
                    identification.genre.as_deref().unwrap_or(""),
                )
            }
            _ => {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                    <item>\n\
                    \t<title>{}</title>\n\
                    \t<type>{}</type>\n\
                    </item>",
                    identification.title.as_deref().unwrap_or(""),
                    format!("{:?}", identification.content_type),
                )
            }
        };

        std::fs::write(&nfo_path, nfo_content)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write NFO: {}", e)))?;

        Ok(())
    }
}

/// Sanitize a string for use as a filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello: World"), "Hello_ World");
        assert_eq!(sanitize_filename("File/Name"), "File_Name");
        assert_eq!(sanitize_filename("Normal Name"), "Normal Name");
    }
}
