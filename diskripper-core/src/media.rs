use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MediaType {
    Video,
    Audio,
    Image,
    Data,
    Program,
    Archive,
    Document,
    DiscImage,
    Unknown,
}

impl MediaType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Video
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
            | "ts" | "m2ts" | "vob" | "ifo" | "bup" => MediaType::Video,
            // Audio
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "aiff" | "ape"
            | "alac" | "pcm" | "ac3" | "dts" => MediaType::Audio,
            // Image
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tga" | "webp" | "svg" | "ico"
            | "raw" | "cr2" | "nef" | "arw" | "heic" => MediaType::Image,
            // Programs
            "exe" | "msi" | "app" | "pkg" | "deb" | "rpm" | "appimage" | "sh" | "bat" | "cmd"
            | "ps1" | "jar" | "py" | "js" | "cpp" | "c" | "h" | "rs" | "go" | "rb" | "php" => {
                MediaType::Program
            }
            // Disc images (checked before archive to take priority)
            "nrg" | "mdf" | "mds" | "vcd" | "svcd" => MediaType::DiscImage,
            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "lz" | "lzma" | "zst" | "cab"
            | "iso" | "img" | "bin" | "cue" | "dmg" | "vhd" | "vmdk" => MediaType::Archive,
            // Documents
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt"
            | "ods" | "odp" | "epub" | "mobi" | "azw" | "azw3" => MediaType::Document,
            _ => MediaType::Unknown,
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            MediaType::Video => "🎬",
            MediaType::Audio => "🎵",
            MediaType::Image => "🖼️",
            MediaType::Data => "💾",
            MediaType::Program => "⚙️",
            MediaType::Archive => "📦",
            MediaType::Document => "📄",
            MediaType::DiscImage => "💿",
            MediaType::Unknown => "❓",
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaType::Video => write!(f, "Video"),
            MediaType::Audio => write!(f, "Audio"),
            MediaType::Image => write!(f, "Image"),
            MediaType::Data => write!(f, "Data"),
            MediaType::Program => write!(f, "Program"),
            MediaType::Archive => write!(f, "Archive"),
            MediaType::Document => write!(f, "Document"),
            MediaType::DiscImage => write!(f, "Disc Image"),
            MediaType::Unknown => write!(f, "Unknown"),
        }
    }
}

pub fn detect_media_types(files: &[super::types::FileEntry]) -> Vec<MediaType> {
    let mut types = std::collections::HashSet::new();
    for file in files {
        if !file.is_dir {
            let ext = std::path::Path::new(&file.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            types.insert(MediaType::from_extension(ext));
        }
    }
    types.into_iter().collect()
}
