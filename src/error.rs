use thiserror::Error;

/// 错误类型定义
///
/// 详细的错误分类，支持错误码转换用于C FFI调用。
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FjiffyldgError {
    /// 文件不存在或未加载
    #[error("File does not exist or was never loaded")]
    FileNotLoaded,

    /// 文件内容或属性无法访问（权限、被占用等）
    #[error("File content or attributes inaccessible")]
    FileInaccessible,

    /// 文件流读取错误（磁盘故障等）
    #[error("File stream error")]
    StreamError,

    /// 写入不完整（预期大小与实际大小不匹配）
    #[error("Incomplete write: size mismatch")]
    IncompleteWrite,

    /// 内存映射失败
    #[error("Memory-mapped file error")]
    MmapError,

    /// 无效的文件偏移量
    #[error("Invalid file offset")]
    InvalidOffset,

    /// 无效的行索引
    #[error("Invalid line index")]
    InvalidLineIndex,

    /// 缓冲区太小
    #[error("Buffer size too small")]
    BufferTooSmall,

    /// 编码检测失败
    #[error("Encoding detection failed")]
    EncodingError,

    /// 通用IO错误
    #[error("IO error")]
    IoError,
}

impl FjiffyldgError {
    /// 转换为错误码（C兼容）
    ///
    /// # 错误码说明
    /// - `0`：无错误
    /// - `-1`：文件不存在/未加载
    /// - `1`：文件无法访问
    /// - `2`：流读取错误
    /// - `3`：内存映射失败
    /// - `4`：无效的文件偏移
    /// - `5`：无效的行索引
    /// - `6`：缓冲区太小
    /// - `7`：编码错误
    /// - `-2`：IO错误
    pub fn to_error_code(&self) -> i32 {
        match self {
            FjiffyldgError::FileNotLoaded => -1,
            FjiffyldgError::FileInaccessible => 1,
            FjiffyldgError::StreamError => 2,
            FjiffyldgError::MmapError => 3,
            FjiffyldgError::IncompleteWrite => 1,
            FjiffyldgError::InvalidOffset => 4,
            FjiffyldgError::InvalidLineIndex => 5,
            FjiffyldgError::BufferTooSmall => 6,
            FjiffyldgError::EncodingError => 7,
            FjiffyldgError::IoError => -2,
        }
    }

    /// 从 C 兼容错误码还原为错误类型
    ///
    /// 返回 `None` 表示错误码为 `0` 或未知值。
    pub fn from_error_code(code: i32) -> Option<Self> {
        match code {
            -1 => Some(FjiffyldgError::FileNotLoaded),
            1 => Some(FjiffyldgError::FileInaccessible),
            2 => Some(FjiffyldgError::StreamError),
            3 => Some(FjiffyldgError::MmapError),
            // IncompleteWrite also maps to 1, but FileInaccessible
            // is the canonical reverse mapping for load_status().
            4 => Some(FjiffyldgError::InvalidOffset),
            5 => Some(FjiffyldgError::InvalidLineIndex),
            6 => Some(FjiffyldgError::BufferTooSmall),
            7 => Some(FjiffyldgError::EncodingError),
            -2 => Some(FjiffyldgError::IoError),
            _ => None,
        }
    }
}

/// 标准返回值类型
pub type Result<T> = std::result::Result<T, FjiffyldgError>;

/// UTF编码模式
///
/// 用于指定文件中使用的宽字符编码。
/// 用于行扫描时的多字节字符处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtfMode {
    /// 默认编码（UTF-8 或 ASCII）
    Default = 0,
    /// UTF-16 小端字节序
    Utf16Le = 1,
    /// UTF-16 大端字节序
    Utf16Be = 2,
    /// UTF-32 小端字节序
    Utf32Le = 3,
    /// UTF-32 大端字节序
    Utf32Be = 4,
}
