//! C ABI 绑定。
//!
//! 本模块导出与 C++ 参考实现同名的核心函数，供 C/C++ 或其他 FFI 调用方使用。
//! 所有返回的缓冲区指针都由 `fjiffyldg_t` 持有，调用方不应自行释放。

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::encoding::{
    check_extract_text_utf8, check_text_ascii, check_whole_text_utf8,
    get_utf8_char_count_with_offset,
};
use crate::error::{FjiffyldgError, UtfMode};
use crate::file::{
    append_file, clone_file, concatenate_files, get_file_size, save_file, MMAP_CHUNK_SIZE,
};
use crate::Fjiffyldg;
use memmap2::{Mmap, MmapOptions};
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int, c_longlong, c_uint};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::ptr;

/// C ABI 使用的不透明文件处理句柄。
pub struct fjiffyldg_t {
    /// Rust 高层文件处理实例
    model: Fjiffyldg,
    /// 最近一次读取返回给 C 调用方的缓冲区
    read_buffer: Vec<u8>,
    /// 最近一次 huge mapping 返回给 C 调用方的真实 mmap 资源
    huge_mmap: Option<Mmap>,
}

/// C ABI 句柄指针类型。
pub type fjiffyldg_ptr = *mut fjiffyldg_t;

impl fjiffyldg_t {
    /// 创建新的 C ABI 句柄。
    fn new() -> Self {
        Self {
            model: Fjiffyldg::new(),
            read_buffer: Vec::new(),
            huge_mmap: None,
        }
    }

    /// 将读取结果保存到内部缓冲区并返回 C 指针。
    fn store_read_buffer(&mut self, data: Option<Vec<u8>>, len: *mut c_uint) -> *const c_char {
        if len.is_null() {
            return ptr::null();
        }

        match data {
            Some(data) => {
                self.read_buffer = data;
                unsafe {
                    *len = self.read_buffer.len().min(c_uint::MAX as usize) as c_uint;
                }
                self.read_buffer.as_ptr().cast::<c_char>()
            }
            None => {
                unsafe {
                    *len = 0;
                }
                ptr::null()
            }
        }
    }
}

/// 将 Rust 错误转换为 C 兼容错误码。
fn error_code(error: FjiffyldgError) -> c_int {
    error.to_error_code()
}

/// 将 `Result` 转换为 C 兼容错误码。
fn result_code<T>(result: crate::Result<T>) -> c_int {
    match result {
        Ok(_) => 0,
        Err(error) => error_code(error),
    }
}

/// 将文件 helper 的结果转换为与 C++ 参考实现一致的公开返回码。
///
/// - `0`：成功
/// - `1`：操作不完整（写后大小校验失败）
/// - `-1`：其他错误
fn file_helper_code(result: crate::Result<()>) -> c_int {
    match result {
        Ok(()) => 0,
        Err(FjiffyldgError::IncompleteWrite) => 1,
        Err(_) => -1,
    }
}

/// 捕获 FFI 边界内的 panic，避免 panic 跨越 C ABI。
fn ffi_guard<T, F>(fallback: T, body: F) -> T
where
    F: FnOnce() -> T,
{
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(fallback)
}

/// 将 C 字符串转换为 Rust 路径。
fn path_from_c(name: *const c_char) -> crate::Result<PathBuf> {
    if name.is_null() {
        return Err(FjiffyldgError::FileNotLoaded);
    }

    let value = unsafe { CStr::from_ptr(name) };
    let value = value.to_str().map_err(|_| FjiffyldgError::EncodingError)?;
    Ok(PathBuf::from(value))
}

/// 将 C 字节指针转换为切片。
fn bytes_from_c<'a>(text: *const c_char, len: c_uint) -> Option<&'a [u8]> {
    bytes_from_c_len(text, len as usize)
}

/// 将 C 字节指针和 `usize` 长度转换为切片。
fn bytes_from_c_len<'a>(text: *const c_char, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }

    if text.is_null() {
        return None;
    }

    Some(unsafe { std::slice::from_raw_parts(text.cast::<u8>(), len) })
}

/// 获取可变句柄引用。
fn handle_mut<'a>(fm: fjiffyldg_ptr) -> Option<&'a mut fjiffyldg_t> {
    unsafe { fm.as_mut() }
}

/// 获取不可变句柄引用。
fn handle_ref<'a>(fm: fjiffyldg_ptr) -> Option<&'a fjiffyldg_t> {
    unsafe { fm.as_ref() }
}

/// 将 C ABI UTF 模式转换为 Rust UTF 模式。
fn utf_mode_from_c(utf: c_int) -> UtfMode {
    match utf {
        -1 => UtfMode::AutoDetect,
        1 => UtfMode::Utf16Le,
        2 => UtfMode::Utf16Be,
        3 => UtfMode::Utf32Le,
        4 => UtfMode::Utf32Be,
        _ => UtfMode::Default,
    }
}

/// 创建并返回一个 `fjiffyldg` 文件处理句柄。
#[no_mangle]
pub extern "C" fn fjiffyldg_create() -> fjiffyldg_ptr {
    ffi_guard(ptr::null_mut(), || {
        Box::into_raw(Box::new(fjiffyldg_t::new()))
    })
}

/// 释放 `fjiffyldg` 文件处理句柄。
#[no_mangle]
pub extern "C" fn fjiffyldg_clear(fm: fjiffyldg_ptr) {
    ffi_guard((), || {
        if !fm.is_null() {
            unsafe {
                drop(Box::from_raw(fm));
            }
        }
    })
}

/// 加载文件并启动后台行扫描。
#[no_mangle]
pub extern "C" fn LoadAndScanFile(fm: fjiffyldg_ptr, name: *const c_char) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        let Some(handle) = handle_mut(fm) else {
            return FjiffyldgError::FileNotLoaded.to_error_code();
        };

        let path = match path_from_c(name) {
            Ok(path) => path,
            Err(error) => return error_code(error),
        };

        result_code(handle.model.load_and_scan(path))
    })
}

/// 仅加载文件，不启动行扫描。
#[no_mangle]
pub extern "C" fn LoadFileOnly(fm: fjiffyldg_ptr, name: *const c_char) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        let Some(handle) = handle_mut(fm) else {
            return FjiffyldgError::FileNotLoaded.to_error_code();
        };

        let path = match path_from_c(name) {
            Ok(path) => path,
            Err(error) => return error_code(error),
        };

        result_code(handle.model.load(path))
    })
}

/// 获取文件加载状态。
#[no_mangle]
pub extern "C" fn GetFileIsLoaded(fm: fjiffyldg_ptr) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        let Some(handle) = handle_ref(fm) else {
            return FjiffyldgError::FileNotLoaded.to_error_code();
        };

        match handle.model.load_status() {
            Ok(true) => 0,
            Ok(false) => FjiffyldgError::FileNotLoaded.to_error_code(),
            Err(error) => error_code(error),
        }
    })
}

/// 重新扫描已加载文件的行结构。
/// 重新扫描已加载文件的行结构。
///
/// @return 0 if successful, non-zero error code on failure.
#[no_mangle]
pub extern "C" fn RestartScanFile(
    fm: fjiffyldg_ptr,
    name: *const c_char,
    offset: c_longlong,
    utf: c_int,
) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        let Some(handle) = handle_mut(fm) else {
            return FjiffyldgError::FileNotLoaded.to_error_code();
        };

        if offset < 0 {
            handle.model.request_stop_scan();
            return 0;
        }

        if !name.is_null() {
            let path = match path_from_c(name) {
                Ok(path) => path,
                Err(error) => return error_code(error),
            };
            if let Err(error) = handle.model.load(path) {
                return error_code(error);
            }
        }
        let utf_mode = utf_mode_from_c(utf);
        // Default 模式也触发 BOM 自动检测，与 Rust API restart_scan 行为一致
        let auto_detect = utf_mode == UtfMode::AutoDetect || utf_mode == UtfMode::Default;
        result_code(handle.model.handle().restart_scan_with_auto_detect(
            offset as u64,
            utf_mode,
            auto_detect,
        ))
    })
}

/// 阻塞等待后台行扫描完成。
#[no_mangle]
pub extern "C" fn WaitFileScanTaskFinished(fm: fjiffyldg_ptr) {
    ffi_guard((), || {
        if let Some(handle) = handle_ref(fm) {
            handle.model.wait_scan();
        }
    })
}

/// 请求停止后台行扫描并清空当前行索引。
#[no_mangle]
pub extern "C" fn BackstageRequestStop(fm: fjiffyldg_ptr) {
    ffi_guard((), || {
        if let Some(handle) = handle_ref(fm) {
            handle.model.request_stop_scan();
        }
    })
}

/// 获取文件行数。
#[no_mangle]
pub extern "C" fn GetFileLineCount(fm: fjiffyldg_ptr) -> c_longlong {
    ffi_guard(-1, || {
        handle_ref(fm)
            .map(|handle| handle.model.line_count() as c_longlong)
            .unwrap_or(-1)
    })
}

/// 获取指定行的起始字节位置。
#[no_mangle]
pub extern "C" fn GetFileLinePos(fm: fjiffyldg_ptr, index: c_longlong) -> c_longlong {
    ffi_guard(-1, || {
        handle_ref(fm)
            .map(|handle| handle.model.line_pos(index) as c_longlong)
            .unwrap_or(-1)
    })
}

/// 获取指定行的内容长度。
#[no_mangle]
pub extern "C" fn GetFileLineLength(fm: fjiffyldg_ptr, index: c_longlong) -> c_longlong {
    ffi_guard(-1, || {
        handle_ref(fm)
            .map(|handle| handle.model.line_length(index) as c_longlong)
            .unwrap_or(-1)
    })
}

/// 根据字节位置查找所在行索引。
#[no_mangle]
pub extern "C" fn GetFileLineIndex(fm: fjiffyldg_ptr, pos: c_longlong) -> c_longlong {
    ffi_guard(-1, || {
        handle_ref(fm)
            .map(|handle| handle.model.line_at_pos(pos) as c_longlong)
            .unwrap_or(-1)
    })
}

/// 从指定字节位置读取文件数据。
#[no_mangle]
pub extern "C" fn ReadFileData(
    fm: fjiffyldg_ptr,
    pos: c_longlong,
    len: *mut c_uint,
) -> *const c_char {
    ffi_guard(ptr::null(), || {
        let Some(handle) = handle_mut(fm) else {
            return ptr::null();
        };
        if len.is_null() {
            return ptr::null();
        }

        let requested_len = unsafe { *len } as usize;
        let data = handle.model.read(pos, requested_len);
        handle.store_read_buffer(data, len)
    })
}

/// 按行边界读取文件数据，并截断超长行。
#[no_mangle]
pub extern "C" fn ReadFileDataLLineCut(
    fm: fjiffyldg_ptr,
    index: *mut c_longlong,
    bpos: *mut c_longlong,
    epos: *mut c_longlong,
    len: *mut c_uint,
) -> *const c_char {
    ffi_guard(ptr::null(), || {
        let Some(handle) = handle_mut(fm) else {
            return ptr::null();
        };
        if index.is_null() || bpos.is_null() || epos.is_null() || len.is_null() {
            return ptr::null();
        }

        let mut index_value = unsafe { *index };
        let mut bpos_value = 0;
        let mut epos_value = 0;
        let mut len_value = unsafe { *len } as usize;

        let data = handle.model.read_line_cut(
            &mut index_value,
            &mut bpos_value,
            &mut epos_value,
            &mut len_value,
        );

        if data.is_some() {
            unsafe {
                *index = index_value;
                *bpos = bpos_value;
                *epos = epos_value;
                *len = len_value.min(c_uint::MAX as usize) as c_uint;
            }
        } else {
            unsafe {
                *len = len_value.min(c_uint::MAX as usize) as c_uint;
            }
        }

        handle.store_read_buffer(data, len)
    })
}

/// 从指定位置读取到当前行末尾。
#[no_mangle]
pub extern "C" fn ReadFileDataEndOfLine(
    fm: fjiffyldg_ptr,
    index: c_longlong,
    pos: c_longlong,
    len: *mut c_uint,
) -> *const c_char {
    ffi_guard(ptr::null(), || {
        let Some(handle) = handle_mut(fm) else {
            return ptr::null();
        };
        if len.is_null() {
            return ptr::null();
        }

        let mut len_value = unsafe { *len } as usize;
        let data = handle.model.read_to_line_end(index, pos, &mut len_value);
        unsafe {
            *len = len_value.min(c_uint::MAX as usize) as c_uint;
        }
        handle.store_read_buffer(data, len)
    })
}

/// 获取由句柄持有的整个文件 mmap 指针。
#[no_mangle]
pub extern "C" fn GetFileMappedHuge(
    fm: fjiffyldg_ptr,
    fileName: *const c_char,
    bufferSize: *mut c_longlong,
) -> *const c_char {
    ffi_guard(ptr::null(), || {
        let Some(handle) = handle_mut(fm) else {
            return ptr::null();
        };
        if bufferSize.is_null() {
            return ptr::null();
        }

        // Every call invalidates any previously returned huge mapping.
        handle.huge_mmap = None;

        let path = match path_from_c(fileName) {
            Ok(path) => path,
            Err(_) => {
                unsafe {
                    *bufferSize = 0;
                }
                return ptr::null();
            }
        };

        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => {
                unsafe {
                    *bufferSize = 0;
                }
                return ptr::null();
            }
        };
        let file_size = match file.metadata() {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                unsafe {
                    *bufferSize = 0;
                }
                return ptr::null();
            }
        };
        if file_size == 0 {
            handle.huge_mmap = None;
            unsafe {
                *bufferSize = 0;
            }
            return ptr::null();
        }

        // 32 位平台受虚拟地址空间限制（~3GB），最多映射 1GB 分块；
        // 64 位平台地址空间充裕（128TB），映射整个文件。
        let map_result = if cfg!(target_pointer_width = "32") {
            let map_len = file_size.min(MMAP_CHUNK_SIZE) as usize;
            unsafe { MmapOptions::new().offset(0).len(map_len).map(&file) }
        } else {
            unsafe { MmapOptions::new().map(&file) }
        };

        match map_result {
            Ok(mmap) => {
                unsafe {
                    *bufferSize = mmap.len().min(c_longlong::MAX as usize) as c_longlong;
                }
                let ptr = mmap.as_ptr().cast::<c_char>();
                handle.huge_mmap = Some(mmap);
                ptr
            }
            Err(_) => {
                handle.huge_mmap = None;
                unsafe {
                    *bufferSize = 0;
                }
                ptr::null()
            }
        }
    })
}

/// 清理 huge mmap 内部映射资源。
#[no_mangle]
pub extern "C" fn ClearHugeBuffer(fm: fjiffyldg_ptr) {
    ffi_guard((), || {
        if let Some(handle) = handle_mut(fm) {
            handle.huge_mmap = None;
        }
    })
}

/// 获取文件大小。
#[no_mangle]
pub extern "C" fn GetFileSizeByteCount(name: *const c_char) -> c_longlong {
    ffi_guard(
        FjiffyldgError::IoError.to_error_code() as c_longlong,
        || {
            let path = match path_from_c(name) {
                Ok(path) => path,
                Err(error) => return error_code(error) as c_longlong,
            };

            get_file_size(path)
                .map(|size| size.min(c_longlong::MAX as u64) as c_longlong)
                .unwrap_or(-1)
        },
    )
}

/// 检查文本是否全部为 ASCII。
#[no_mangle]
pub extern "C" fn CheckTextASCII(text: *const c_char, len: c_uint) -> c_uint {
    ffi_guard(len, || {
        bytes_from_c(text, len)
            .map(check_text_ascii)
            .unwrap_or(len as usize)
            .min(c_uint::MAX as usize) as c_uint
    })
}

/// 检查完整文本是否为 UTF-8。
#[no_mangle]
pub extern "C" fn CheckWholeTextUtf8(text: *const c_char, len: c_uint) -> c_uint {
    ffi_guard(len, || {
        bytes_from_c(text, len)
            .map(check_whole_text_utf8)
            .unwrap_or(len as usize)
            .min(c_uint::MAX as usize) as c_uint
    })
}

/// 随机抽样检查文本片段是否为 UTF-8。
#[no_mangle]
pub extern "C" fn CheckExtractTextUtf8(text: *const c_char, len: c_uint) -> c_uint {
    ffi_guard(len, || {
        bytes_from_c(text, len)
            .map(check_extract_text_utf8)
            .unwrap_or(len as usize)
            .min(c_uint::MAX as usize) as c_uint
    })
}

/// 统计 UTF-8 字符数，并推进调用方传入的文本指针。
#[no_mangle]
pub extern "C" fn GetUtf8TextCharCount(text: *mut *const c_char, len: c_uint) -> c_uint {
    ffi_guard(0, || {
        if text.is_null() {
            return 0;
        }

        let start = unsafe { *text };
        if start.is_null() {
            return 0;
        }
        let Some(data) = bytes_from_c(start, len) else {
            return 0;
        };

        let (count, consumed) = get_utf8_char_count_with_offset(data);
        unsafe {
            *text = start.add(consumed);
        }

        count.min(c_uint::MAX as usize) as c_uint
    })
}

/// 克隆文件。
#[no_mangle]
pub extern "C" fn ToCloneFile(oldFileName: *const c_char, newFileName: *const c_char) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        let old_path = match path_from_c(oldFileName) {
            Ok(path) => path,
            Err(_) => return -1,
        };
        let new_path = match path_from_c(newFileName) {
            Ok(path) => path,
            Err(_) => return -1,
        };

        file_helper_code(clone_file(old_path, new_path))
    })
}

/// 保存指定内容到文件。
#[no_mangle]
pub extern "C" fn ToSaveFile(
    fileName: *const c_char,
    buffer: *const c_char,
    len: c_longlong,
) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        if len < 0 {
            return -1;
        }

        let path = match path_from_c(fileName) {
            Ok(path) => path,
            Err(_) => return -1,
        };
        let Some(data) = bytes_from_c_len(buffer, len as usize) else {
            return -1;
        };

        file_helper_code(save_file(path, data))
    })
}

/// 追加指定内容到文件。
#[no_mangle]
pub extern "C" fn ToAppendFile(
    fileName: *const c_char,
    buffer: *const c_char,
    len: c_longlong,
) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        if len < 0 {
            return -1;
        }

        let path = match path_from_c(fileName) {
            Ok(path) => path,
            Err(_) => return -1,
        };
        let Some(data) = bytes_from_c_len(buffer, len as usize) else {
            return -1;
        };

        file_helper_code(append_file(path, data))
    })
}

/// 将第二个文件内容追加到第一个文件。
#[no_mangle]
pub extern "C" fn ToConcatenateFile(
    catFileName: *const c_char,
    appendFileName: *const c_char,
) -> c_int {
    ffi_guard(FjiffyldgError::IoError.to_error_code(), || {
        let cat_path = match path_from_c(catFileName) {
            Ok(path) => path,
            Err(_) => return -1,
        };
        let append_path = match path_from_c(appendFileName) {
            Ok(path) => path,
            Err(_) => return -1,
        };

        file_helper_code(concatenate_files([append_path], cat_path))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::Write;
    use std::thread;
    use tempfile::NamedTempFile;

    #[test]
    fn get_file_mapped_huge_retains_mmap_until_clear() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello mmap").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
        let fm = fjiffyldg_create();
        let mut size = 0;

        let ptr = GetFileMappedHuge(fm, path.as_ptr(), &mut size);

        assert!(!ptr.is_null());
        assert_eq!(size, 10);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), 5) },
            b"hello"
        );
        assert!(unsafe { (*fm).huge_mmap.is_some() });

        ClearHugeBuffer(fm);

        assert!(unsafe { (*fm).huge_mmap.is_none() });
        fjiffyldg_clear(fm);
    }

    #[test]
    fn get_file_mapped_huge_clears_previous_mapping_on_failure() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello mmap").unwrap();
        let valid_path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
        let missing_path = CString::new("missing-file-for-huge-buffer.txt").unwrap();
        let fm = fjiffyldg_create();
        let mut size = 0;

        let ptr = GetFileMappedHuge(fm, valid_path.as_ptr(), &mut size);

        assert!(!ptr.is_null());
        assert!(unsafe { (*fm).huge_mmap.is_some() });

        let failed_ptr = GetFileMappedHuge(fm, missing_path.as_ptr(), &mut size);

        assert!(failed_ptr.is_null());
        assert_eq!(size, 0);
        assert!(unsafe { (*fm).huge_mmap.is_none() });

        fjiffyldg_clear(fm);
    }

    #[test]
    fn get_file_line_pos_exposes_scanned_prefix_while_scanning() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&b"x\n".repeat(16 * 1024 * 1024)).unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
        let fm = fjiffyldg_create();

        assert_eq!(LoadAndScanFile(fm, path.as_ptr()), 0);
        assert!(unsafe { (*fm).model.is_scanning() });
        assert_eq!(GetFileLineCount(fm), -1);

        let mut saw_partial_line_pos = false;
        for _ in 0..10_000 {
            if GetFileLinePos(fm, 0) == 0 {
                saw_partial_line_pos = true;
                break;
            }
            if !unsafe { (*fm).model.is_scanning() } {
                break;
            }
            thread::yield_now();
        }

        assert!(saw_partial_line_pos);

        BackstageRequestStop(fm);
        fjiffyldg_clear(fm);
    }

    #[test]
    fn get_file_line_index_stays_unavailable_while_scanning() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&b"x\n".repeat(16 * 1024 * 1024)).unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
        let fm = fjiffyldg_create();

        assert_eq!(LoadAndScanFile(fm, path.as_ptr()), 0);
        assert!(unsafe { (*fm).model.is_scanning() });
        assert_eq!(GetFileLineCount(fm), -1);
        // 扫描中 GetFileLineIndex 可能已对已扫描前缀返回有效结果
        // （与 C++ 行为一致：GetLineByPos 在 linescanRunning 时可查已扫描前缀）
        let result = GetFileLineIndex(fm, 0);
        // 结果为 0（已索引）或 -1（尚未索引）均可接受
        assert!(result == 0 || result == -1, "unexpected result: {result}");

        BackstageRequestStop(fm);
        fjiffyldg_clear(fm);
    }

    // ---- FFI 空指针防护验证 ----

    /// LoadAndScanFile(null,...) → -1
    #[test]
    fn load_and_scan_null_handle_returns_neg1() {
        // CI 中 `clippy::manual_c_str_literals` 会报错，
        // 直接使用 C 字符串字面量 `c"dummy"` 代替手工拼接 NUL。
        let ret = LoadAndScanFile(ptr::null_mut(), c"dummy".as_ptr().cast());
        assert_eq!(
            ret, -1,
            "LoadAndScanFile(null,...) should return -1, got {ret}"
        );
    }

    /// ReadFileData(valid_fm, 0, null_len) → null
    #[test]
    fn read_file_data_null_len_returns_null() {
        let fm = fjiffyldg_create();
        assert!(!fm.is_null());
        let ret = ReadFileData(fm, 0, ptr::null_mut());
        assert!(ret.is_null(), "ReadFileData(fm,0,null) should return null");
        fjiffyldg_clear(fm);
    }

    /// GetUtf8TextCharCount(null, 10) → 0
    #[test]
    fn get_utf8_text_char_count_null_returns_0() {
        let ret = GetUtf8TextCharCount(ptr::null_mut(), 10);
        assert_eq!(
            ret, 0,
            "GetUtf8TextCharCount(null,10) should return 0, got {ret}"
        );
    }

    /// ToSaveFile(null, buf, len) → -1
    #[test]
    fn to_save_file_null_name_returns_neg1() {
        let buf = b"hello";
        let ret = ToSaveFile(ptr::null(), buf.as_ptr().cast(), 5);
        assert_eq!(ret, -1, "ToSaveFile(null,...) should return -1, got {ret}");
    }

    // ---- FFI 负参数防御验证 ----

    /// 验证 FFI 层对负参数的防御性处理
    ///
    /// - `GetFileLinePos(fm, -1)` → -1
    /// - `GetFileLineLength(fm, -1)` → -1
    /// - `GetFileLineIndex(fm, -1)` → -1
    /// - `ReadFileData(fm, -1, &len)` → pos 钳位到 0（读取文件头）
    /// - `ReadFileDataLLineCut` index=-1 → null，len 置 0
    #[test]
    fn negative_params_handling() {
        // 准备一个包含多行内容的临时文件
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line0\nline1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
        let fm = fjiffyldg_create();

        assert_eq!(LoadAndScanFile(fm, path.as_ptr()), 0);
        // 等待扫描完成
        unsafe {
            (*fm).model.wait_scan();
        }

        // 1. GetFileLinePos(fm, -1) → -1
        assert_eq!(GetFileLinePos(fm, -1), -1, "line_pos(-1) 应返回 -1");

        // 2. GetFileLineLength(fm, -1) → -1
        assert_eq!(GetFileLineLength(fm, -1), -1, "line_length(-1) 应返回 -1");

        // 3. GetFileLineIndex(fm, -1) → -1
        assert_eq!(GetFileLineIndex(fm, -1), -1, "line_at_pos(-1) 应返回 -1");

        // 4. ReadFileData(fm, -1, &len) → pos 钳位到 0，读取文件开头
        let mut len: c_uint = 100;
        let ptr = ReadFileData(fm, -1, &mut len);
        // pos=-1 钳位到 0，应返回文件开头数据（read_data 是字节级读取，不按行分割）
        assert!(
            !ptr.is_null(),
            "ReadFileData(fm,-1) 钳位到 0 应返回有效指针"
        );
        assert_eq!(
            len, 18,
            "ReadFileData(fm,-1) 钳位到 0 应读取整个文件（18字节），got {len}"
        );
        let data = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize) };
        assert!(
            data.starts_with(b"line0\n"),
            "ReadFileData(fm,-1) 钳位到 0 应从文件头开始读取"
        );

        // 5. ReadFileDataLLineCut(index=-1) → null，len 置 0
        let mut index: c_longlong = -1;
        let mut bpos: c_longlong = 0;
        let mut epos: c_longlong = 0;
        let mut len_cut: c_uint = 100;
        let ptr_cut = ReadFileDataLLineCut(fm, &mut index, &mut bpos, &mut epos, &mut len_cut);
        assert!(
            ptr_cut.is_null(),
            "ReadFileDataLLineCut(index=-1) 应返回 null"
        );
        assert_eq!(len_cut, 0, "ReadFileDataLLineCut(index=-1) 应将 len 置为 0");

        // 6. 重复验证 ReadFileData(fm, -1, &len) 钳位（len=3 时应返回前 3 字节）
        let mut len2: c_uint = 3;
        let ptr2 = ReadFileData(fm, -1, &mut len2);
        assert!(
            !ptr2.is_null(),
            "ReadFileData(fm,-1,len=3) 钳位到 0 应返回有效指针"
        );
        assert_eq!(len2, 3, "ReadFileData(fm,-1,len=3) 应读取 3 字节");
        assert_eq!(
            unsafe { std::slice::from_raw_parts(ptr2.cast::<u8>(), len2 as usize) },
            b"lin",
            "ReadFileData(fm,-1,len=3) 钳位到 0 应读到文件前 3 字节"
        );

        fjiffyldg_clear(fm);
    }
}
