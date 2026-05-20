#include "fjiffyldg.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

int fjiffyldg_c_header_smoke_debug(const char* path) {
    fprintf(stderr, "DEBUG: start\n");
    fjiffyldg_t* handle = fjiffyldg_create();
    fprintf(stderr, "DEBUG: created handle %p\n", (void*)handle);
    uint32_t len = 5;
    int64_t index = 0;
    int64_t begin = 0;
    int64_t end = 0;
    const char* cursor = "hello";
    long long mapped_len = 0;
    const char* mapped = 0;
    const char* data = 0;

    if (handle == 0) {
        fprintf(stderr, "DEBUG: handle null\n");
        return -1;
        }

    fprintf(stderr, "DEBUG: calling LoadAndScanFile(%s)\n", path);
    if (LoadAndScanFile(handle, path) != 0) {
        fprintf(stderr, "DEBUG: LoadAndScanFile failed\n");
        fjiffyldg_clear(handle);
        return -2;
        }

    fprintf(stderr, "DEBUG: waiting scan finish\n");
    WaitFileScanTaskFinished(handle);
    fprintf(stderr, "DEBUG: finished scan\n");
    if (GetFileIsLoaded(handle) != 0) {
        fprintf(stderr, "DEBUG: GetFileIsLoaded != 0\n");
        fjiffyldg_clear(handle);
        return -3;
        }

    long long lineCount = GetFileLineCount(handle);
    fprintf(stderr, "DEBUG: lineCount=%lld\n", lineCount);
    if (lineCount < 3) {
        fprintf(stderr, "DEBUG: insufficient lines\n");
        fjiffyldg_clear(handle);
        return -4;
        }

    long long pos1 = GetFileLinePos(handle, 1);
    long long len0 = GetFileLineLength(handle, 0);
    fprintf(stderr, "DEBUG: pos1=%lld len0=%lld\n", pos1, len0);
    if (pos1 <= 0 || len0 != 5) {
        fprintf(stderr, "DEBUG: line pos/len mismatch pos1=%lld len0=%lld\n", pos1, len0);
        fjiffyldg_clear(handle);
        return -5;
        }

    if (GetFileLineIndex(handle, 6) != 1) {
        fprintf(stderr, "DEBUG: GetFileLineIndex != 1\n");
        fjiffyldg_clear(handle);
        return -6;
        }

    data = ReadFileData(handle, 0, &len);
    fprintf(stderr, "DEBUG: ReadFileData len=%u data=%p\n", len, (void*)data);
    if (data == 0 || len != 5 || memcmp(data, "alpha", 5) != 0) {
        fprintf(stderr, "DEBUG: ReadFileData content mismatch\n");
        fjiffyldg_clear(handle);
        return -7;
        }

    (void)ReadFileDataLLineCut(handle, &index, &begin, &end, &len);
    (void)ReadFileDataEndOfLine(handle, index, begin, &len);

    mapped = GetFileMappedHuge(handle, path, &mapped_len);
    fprintf(stderr, "DEBUG: mapped=%p mapped_len=%lld\n", (void*)mapped, mapped_len);
    if (mapped == 0 || mapped_len <= 0) {
        fprintf(stderr, "DEBUG: GetFileMappedHuge failed\n");
        fjiffyldg_clear(handle);
        return -8;
        }
    ClearHugeBuffer(handle);

    if (GetUtf8TextCharCount(&cursor, 5) != 5 || cursor == 0 || *cursor != '\0') {
        fprintf(stderr, "DEBUG: GetUtf8TextCharCount failed cursor=%p *cursor=%c\n", (void*)cursor, *cursor);
        fjiffyldg_clear(handle);
        return -9;
        }

    if (CheckTextASCII("hello", 5) != 0 || CheckWholeTextUtf8("hello", 5) != 0 ||
        CheckExtractTextUtf8("hello", 5) != 0) {
        fprintf(stderr, "DEBUG: text checks failed\n");
        fjiffyldg_clear(handle);
        return -10;
        }

    BackstageRequestStop(handle);
    fjiffyldg_clear(handle);

    fprintf(stderr, "DEBUG: success\n");
    return 0;
    }

#ifdef FJIFFYLDG_SMOKE_MAIN
int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: c_smoke_debug <input-file>\n");
        return 64;
        }

    return fjiffyldg_c_header_smoke_debug(argv[1]);
    }
#endif
