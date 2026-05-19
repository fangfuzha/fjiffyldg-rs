#ifndef FJIFFYLDG_H
#define FJIFFYLDG_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque file processing handle owned by the Fjiffyldg library. */
typedef struct fjiffyldg_t fjiffyldg_t;

/** Creates a new file processing handle. */
fjiffyldg_t *fjiffyldg_create(void);

/** Releases a file processing handle created by fjiffyldg_create(). */
void fjiffyldg_clear(fjiffyldg_t *fm);

/** Loads a file and starts background line scanning. */
int LoadAndScanFile(fjiffyldg_t *fm, const char *name);

/** Loads a file without starting line scanning. */
int LoadFileOnly(fjiffyldg_t *fm, const char *name);

/** Returns 0 when the file is loaded, otherwise an error code. */
int GetFileIsLoaded(fjiffyldg_t *fm);

/** Restarts line scanning from offset with the selected UTF mode. */
void RestartScanFile(fjiffyldg_t *fm, const char *name, int64_t offset, int utf);

/** Blocks until the background line scan has finished. */
void WaitFileScanTaskFinished(fjiffyldg_t *fm);

/** Requests background scanning to stop and clears the current line index. */
void BackstageRequestStop(fjiffyldg_t *fm);

/** Returns the total line count, or -1 when unavailable. */
int64_t GetFileLineCount(fjiffyldg_t *fm);

/** Returns the byte offset of a line, or -1 when unavailable. */
int64_t GetFileLinePos(fjiffyldg_t *fm, int64_t index);

/** Returns the byte length of a line without its line ending. */
int64_t GetFileLineLength(fjiffyldg_t *fm, int64_t index);

/** Returns the line index containing the given byte position. */
int64_t GetFileLineIndex(fjiffyldg_t *fm, int64_t pos);

/** Reads bytes from a file position; len is both input and output length. */
const char *ReadFileData(fjiffyldg_t *fm, int64_t pos, uint32_t *len);

/** Reads batched line data and truncates long lines. */
const char *ReadFileDataLLineCut(
    fjiffyldg_t *fm,
    int64_t *index,
    int64_t *bpos,
    int64_t *epos,
    uint32_t *len);

/** Reads from a byte position to the current line end. */
const char *ReadFileDataEndOfLine(fjiffyldg_t *fm, int64_t index, int64_t pos, uint32_t *len);

/** Returns a file data copy held by the handle until the next clear call. */
const char *GetFileMappedHuge(fjiffyldg_t *fm, const char *fileName, int64_t *bufferSize);

/** Clears the internal huge-buffer copy. */
void ClearHugeBuffer(fjiffyldg_t *fm);

/** Returns the size of a file in bytes, or an error code. */
int64_t GetFileSizeByteCount(const char *name);

/** Returns 0 when the full input is ASCII; otherwise returns a failing offset. */
uint32_t CheckTextASCII(const char *text, uint32_t len);

/** Returns 0 when the full input is valid UTF-8; otherwise returns a failing offset. */
uint32_t CheckWholeTextUtf8(const char *text, uint32_t len);

/** Checks sampled text ranges for UTF-8 validity. */
uint32_t CheckExtractTextUtf8(const char *text, uint32_t len);

/** Counts UTF-8 characters and advances the caller-provided pointer by consumed bytes. */
uint32_t GetUtf8TextCharCount(const char **text, uint32_t len);

/** Copies a file. */
int ToCloneFile(const char *oldFileName, const char *newFileName);

/** Saves bytes to a file. */
int ToSaveFile(const char *fileName, const char *buffer, int64_t len);

/** Appends bytes to a file. */
int ToAppendFile(const char *fileName, const char *buffer, int64_t len);

/** Appends appendFileName contents to catFileName. */
int ToConcatenateFile(const char *catFileName, const char *appendFileName);

#ifdef __cplusplus
}
#endif

#endif /* FJIFFYLDG_H */
