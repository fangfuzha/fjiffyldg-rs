#include "fjiffyldg.h"

#include <stdio.h>
#include <string.h>

/**
 * Compiles and optionally runs a minimal C caller against the public header.
 *
 * Object-only builds verify declarations. Executable builds additionally verify
 * that the generated header links to and calls the Rust dynamic library.
 */
int fjiffyldg_c_header_smoke(const char* path) {
    fjiffyldg_t* handle = fjiffyldg_create();
    uint32_t len = 5;
    long long index = 0;
    long long begin = 0;
    long long end = 0;
    const char* cursor = "hello";
    long long mapped_len = 0;
    const char* mapped = 0;
    const char* data = 0;

    if (handle == 0) {
        return -1;
        }

    if (LoadAndScanFile(handle, path) != 0) {
        fjiffyldg_clear(handle);
        return -2;
        }

    WaitFileScanTaskFinished(handle);
    if (GetFileIsLoaded(handle) != 0) {
        fjiffyldg_clear(handle);
        return -3;
        }

    if (GetFileLineCount(handle) < 3) {
        fjiffyldg_clear(handle);
        return -4;
        }

    if (GetFileLinePos(handle, 1) <= 0 || GetFileLineLength(handle, 0) != 5) {
        fjiffyldg_clear(handle);
        return -5;
        }

    if (GetFileLineIndex(handle, 6) != 1) {
        fjiffyldg_clear(handle);
        return -6;
        }

    data = ReadFileData(handle, 0, &len);
    if (data == 0 || len != 5 || memcmp(data, "alpha", 5) != 0) {
        fjiffyldg_clear(handle);
        return -7;
        }

    (void)ReadFileDataLLineCut(handle, &index, &begin, &end, &len);
    (void)ReadFileDataEndOfLine(handle, index, begin, &len);

    mapped = GetFileMappedHuge(handle, path, &mapped_len);
    if (mapped == 0 || mapped_len <= 0) {
        fjiffyldg_clear(handle);
        return -8;
        }
    ClearHugeBuffer(handle);

    if (GetUtf8TextCharCount(&cursor, 5) != 5 || cursor == 0 || *cursor != '\0') {
        fjiffyldg_clear(handle);
        return -9;
        }

    if (CheckTextASCII("hello", 5) != 0 || CheckWholeTextUtf8("hello", 5) != 0 ||
        CheckExtractTextUtf8("hello", 5) != 0) {
        fjiffyldg_clear(handle);
        return -10;
        }

    BackstageRequestStop(handle);
    fjiffyldg_clear(handle);

    return 0;
    }

#ifdef FJIFFYLDG_SMOKE_MAIN
int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: c_smoke <input-file>\n");
        return 64;
        }

    return fjiffyldg_c_header_smoke(argv[1]);
    }
#endif
