

/// Image format readers (NRG, MDF/MDS, CCD/IMG/SUB)
#[derive(Debug)]
pub struct ImageFormatReader;

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Iso,
    BinCue,
    NrG,
    MdfMds,
    CcdImgSub,
}

impl ImageFormatReader {
    pub fn new() -> Self {
        Self
    }
}
