//! 错误类型与 UTF 编码模式定义。
//!
//! 本模块定义了 Fjiffyldg 库的所有错误类型（[`FjiffyldgError`]）和 UTF 编码模式（[`UtfMode`]）。
//!
//! # 错误码对照表
//!
//! | 错误码 | 错误类型 | 含义 |
//! |--------|----------|------|
//! | `0` | — | 成功 |
//! | `-1` | [`FjiffyldgError::FileNotLoaded`] | 文件不存在或未加载 |
//! | `1` | [`FjiffyldgError::FileInaccessible`] | 文件不可访问 |
//! | `2` | [`FjiffyldgError::StreamError`] | 流读取错误 |
//! | `3` | [`FjiffyldgError::MmapError`] | 内存映射失败 |
//! | `4` | [`FjiffyldgError::InvalidOffset`] | 无效文件偏移 |
//! | `5` | [`FjiffyldgError::InvalidLineIndex`] | 无效行索引 |
//! | `6` | [`FjiffyldgError::BufferTooSmall`] | 缓冲区太小 |
//! | `7` | [`FjiffyldgError::EncodingError`] | 编码错误 |
//! | `8` | [`FjiffyldgError::IncompleteWrite`] | 写入不完整 |
//! | `-2` | [`FjiffyldgError::IoError`] | 通用 IO 错误 |
//!
//! # 示例
//!
//! ```
//! use fjiffyldg::{FjiffyldgError, UtfMode};
//!
//! // 错误码转换
//! assert_eq!(FjiffyldgError::FileNotLoaded.to_error_code(), -1);
//! assert_eq!(FjiffyldgError::from_error_code(2), Some(FjiffyldgError::StreamError));
//!
//! // UTF 模式
//! let mode = UtfMode::Utf16Le;
//! assert_eq!(mode as i32, 1);
//! ```

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
    /// - `8`：写入不完整（大小校验失败）
    /// - `-2`：IO错误
    pub fn to_error_code(&self) -> i32 {
        match self {
            FjiffyldgError::FileNotLoaded => -1,
            FjiffyldgError::FileInaccessible => 1,
            FjiffyldgError::StreamError => 2,
            FjiffyldgError::MmapError => 3,
            FjiffyldgError::IncompleteWrite => 8,
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
            4 => Some(FjiffyldgError::InvalidOffset),
            5 => Some(FjiffyldgError::InvalidLineIndex),
            6 => Some(FjiffyldgError::BufferTooSmall),
            7 => Some(FjiffyldgError::EncodingError),
            8 => Some(FjiffyldgError::IncompleteWrite),
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
    /// 默认编码模式。在公开 API（`restart_scan`）中等同于
    /// [`AutoDetect`](UtfMode::AutoDetect)：自动根据 BOM 头检测编码，
    /// 未检测到 BOM 时按单字节（ASCII/UTF-8）处理。
    Default = 0,
    /// UTF-16 小端字节序
    Utf16Le = 1,
    /// UTF-16 大端字节序
    Utf16Be = 2,
    /// UTF-32 小端字节序
    Utf32Le = 3,
    /// UTF-32 大端字节序
    Utf32Be = 4,
    /// 自动检测编码（根据 BOM 头）
    AutoDetect = -1,
}
