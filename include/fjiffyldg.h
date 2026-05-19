#ifndef FJIFFYLDG_H
#define FJIFFYLDG_H

#include <stdint.h>

#ifdef FJIFFYLDG_SHARED
#if defined(_WIN32) || defined(__CYGWIN__)
#if defined(BUILDING_FJIFFYLDG)
#define FJIFFYLDG_API __declspec(dllexport)
#else
#define FJIFFYLDG_API __declspec(dllimport)
#endif
#elif defined(__GNUC__) || defined(__clang__)
#define FJIFFYLDG_API __attribute__((visibility("default")))
#else
#define FJIFFYLDG_API
#endif
#else
#define FJIFFYLDG_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque file processing handle owned by the Fjiffyldg library. */
typedef struct fjiffyldg_t fjiffyldg_t;

/** Reference-compatible opaque handle pointer alias. */
typedef struct fjiffyldg_t *fjiffyldg_ptr;

/** Creates a new file processing handle. */
FJIFFYLDG_API fjiffyldg_ptr fjiffyldg_create(void);

/** Releases a file processing handle created by fjiffyldg_create(). */
FJIFFYLDG_API void fjiffyldg_clear(fjiffyldg_ptr fm);

/** Loads a file and starts background line scanning. */
FJIFFYLDG_API int LoadAndScanFile(fjiffyldg_ptr fm, const char *name);

/** Loads a file without starting line scanning. */
FJIFFYLDG_API int LoadFileOnly(fjiffyldg_ptr fm, const char *name);

/** Returns 0 when the file is loaded, otherwise an error code. */
FJIFFYLDG_API int GetFileIsLoaded(fjiffyldg_ptr fm);

/** Restarts line scanning from offset with the selected UTF mode. */
FJIFFYLDG_API void RestartScanFile(fjiffyldg_ptr fm, const char *name, int64_t offset, int utf);

/** Blocks until the background line scan has finished. */
FJIFFYLDG_API void WaitFileScanTaskFinished(fjiffyldg_ptr fm);

/** Requests background scanning to stop and clears the current line index. */
FJIFFYLDG_API void BackstageRequestStop(fjiffyldg_ptr fm);

/** Returns the total line count, or -1 when unavailable. */
FJIFFYLDG_API int64_t GetFileLineCount(fjiffyldg_ptr fm);

/** Returns the byte offset of a line, or -1 when unavailable. */
FJIFFYLDG_API int64_t GetFileLinePos(fjiffyldg_ptr fm, int64_t index);

/** Returns the byte length of a line without its line ending. */
FJIFFYLDG_API int64_t GetFileLineLength(fjiffyldg_ptr fm, int64_t index);

/** Returns the line index containing the given byte position. */
FJIFFYLDG_API int64_t GetFileLineIndex(fjiffyldg_ptr fm, int64_t pos);

/** Reads bytes from a file position; len is both input and output length. */
FJIFFYLDG_API const char *ReadFileData(fjiffyldg_ptr fm, int64_t pos, uint32_t *len);

/** Reads batched line data and truncates long lines. */
FJIFFYLDG_API const char *ReadFileDataLLineCut(
    fjiffyldg_ptr fm,
    int64_t *index,
    int64_t *bpos,
    int64_t *epos,
    uint32_t *len);

/** Reads from a byte position to the current line end. */
FJIFFYLDG_API const char *ReadFileDataEndOfLine(fjiffyldg_ptr fm, int64_t index, int64_t pos, uint32_t *len);

/** Returns a file mmap pointer held by the handle until ClearHugeBuffer or clear. */
FJIFFYLDG_API const char *GetFileMappedHuge(fjiffyldg_ptr fm, const char *fileName, int64_t *bufferSize);

/** Clears the internal huge mmap resource. */
FJIFFYLDG_API void ClearHugeBuffer(fjiffyldg_ptr fm);

/** Returns the size of a file in bytes, or an error code. */
FJIFFYLDG_API int64_t GetFileSizeByteCount(const char *name);

/** Returns 0 when the full input is ASCII; otherwise returns a failing offset. */
FJIFFYLDG_API uint32_t CheckTextASCII(const char *text, uint32_t len);

/** Returns 0 when the full input is valid UTF-8; otherwise returns a failing offset. */
FJIFFYLDG_API uint32_t CheckWholeTextUtf8(const char *text, uint32_t len);

/** Checks sampled text ranges for UTF-8 validity. */
FJIFFYLDG_API uint32_t CheckExtractTextUtf8(const char *text, uint32_t len);

/** Counts UTF-8 characters and advances the caller-provided pointer by consumed bytes. */
FJIFFYLDG_API uint32_t GetUtf8TextCharCount(const char **text, uint32_t len);

/** Copies a file. */
FJIFFYLDG_API int ToCloneFile(const char *oldFileName, const char *newFileName);

/** Saves bytes to a file. */
FJIFFYLDG_API int ToSaveFile(const char *fileName, const char *buffer, int64_t len);

/** Appends bytes to a file. */
FJIFFYLDG_API int ToAppendFile(const char *fileName, const char *buffer, int64_t len);

/** Appends appendFileName contents to catFileName. */
FJIFFYLDG_API int ToConcatenateFile(const char *catFileName, const char *appendFileName);

#ifdef __cplusplus
}

namespace Fjiffyldg {
class Fjiffyldg {
public:
    Fjiffyldg() : handle_(fjiffyldg_create()) {}

    Fjiffyldg(Fjiffyldg &&other) noexcept : handle_(other.handle_) { other.handle_ = nullptr; }

    Fjiffyldg &operator=(Fjiffyldg &&other) noexcept {
        if (this != &other) {
            fjiffyldg_clear(handle_);
            handle_ = other.handle_;
            other.handle_ = nullptr;
        }
        return *this;
    }

    Fjiffyldg(const Fjiffyldg &) = delete;
    Fjiffyldg &operator=(const Fjiffyldg &) = delete;

    ~Fjiffyldg() { fjiffyldg_clear(handle_); }

    fjiffyldg_ptr GetFjiffyldgHandle() { return handle_; }

private:
    fjiffyldg_ptr handle_;
};
}
#endif

#endif /* FJIFFYLDG_H */
