

/// DVD IFO parser for title/chapter extraction
pub mod dvd;

/// Image format readers (NRG, MDF/MDS, CCD/IMG/SUB)
pub mod image_formats;

/// CD-Text/ISRC/MCN extraction via subchannel
pub mod cd_text;

pub use dvd::DvdParser;
pub use image_formats::{ImageFormatReader, ImageFormat};
pub use cd_text::CdTextReader;
