use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FjiffyldgError {
    #[error("File does not exist or was never loaded")]
    FileNotLoaded,
    
    #[error("File content or attributes inaccessible")]
    FileInaccessible,
    
    #[error("File stream error")]
    StreamError,
    
    #[error("Memory-mapped file error")]
    MmapError,
    
    #[error("Invalid file offset")]
    InvalidOffset,
    
    #[error("Invalid line index")]
    InvalidLineIndex,
    
    #[error("Buffer size too small")]
    BufferTooSmall,
    
    #[error("Encoding detection failed")]
    EncodingError,
    
    #[error("IO error")]
    IoError,
}

impl FjiffyldgError {
    pub fn to_error_code(&self) -> i32 {
        match self {
            FjiffyldgError::FileNotLoaded => -1,
            FjiffyldgError::FileInaccessible => 1,
            FjiffyldgError::StreamError => 2,
            FjiffyldgError::MmapError => 3,
            FjiffyldgError::InvalidOffset => 4,
            FjiffyldgError::InvalidLineIndex => 5,
            FjiffyldgError::BufferTooSmall => 6,
            FjiffyldgError::EncodingError => 7,
            FjiffyldgError::IoError => -2,
        }
    }
}

pub type Result<T> = std::result::Result<T, FjiffyldgError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtfMode {
    Default = 0,
    Utf16Le = 1,
    Utf16Be = 2,
    Utf32Le = 3,
    Utf32Be = 4,
    Auto = -1,
}

impl Default for UtfMode {
    fn default() -> Self {
        UtfMode::Default
    }
}