#include "fjiffyldg.h"

#include <stdint.h>

/**
 * Compiles a minimal C caller against the public header.
 *
 * The check intentionally builds as an object file only, so it verifies C ABI
 * declarations without requiring platform-specific dynamic library paths.
 */
int fjiffyldg_c_header_smoke(const char *path) {
    fjiffyldg_t *handle = fjiffyldg_create();
    uint32_t len = 5;
    int64_t index = 0;
    int64_t begin = 0;
    int64_t end = 0;
    const char *cursor = "hello";

    if (handle == 0) {
        return -1;
    }

    if (LoadAndScanFile(handle, path) != 0) {
        fjiffyldg_clear(handle);
        return -2;
    }

    WaitFileScanTaskFinished(handle);
    (void)GetFileIsLoaded(handle);
    (void)GetFileLineCount(handle);
    (void)GetFileLinePos(handle, 0);
    (void)GetFileLineLength(handle, 0);
    (void)GetFileLineIndex(handle, 0);
    (void)ReadFileData(handle, 0, &len);
    (void)ReadFileDataLLineCut(handle, &index, &begin, &end, &len);
    (void)ReadFileDataEndOfLine(handle, index, begin, &len);
    (void)GetUtf8TextCharCount(&cursor, 5);
    (void)CheckTextASCII("hello", 5);
    (void)CheckWholeTextUtf8("hello", 5);
    (void)CheckExtractTextUtf8("hello", 5);
    BackstageRequestStop(handle);
    ClearHugeBuffer(handle);
    fjiffyldg_clear(handle);

    return 0;
}
