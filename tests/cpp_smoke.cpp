#include "fjiffyldg.h"

#include <cstdio>

/**
 * Compiles and optionally runs a minimal C++ caller against the generated public header.
 *
 * This validates the C++ convenience wrapper injected by cbindgen configuration
 * and, in executable mode, verifies that it links to the Rust dynamic library.
 */
int fjiffyldg_cpp_header_smoke(const char* path) {
  Fjiffyldg::Fjiffyldg model;
  fjiffyldg_ptr handle = model.GetFjiffyldgHandle();
  if (handle == nullptr) {
    return -1;
    }

  int code = LoadAndScanFile(handle, path);
  if (code != 0) {
    return code;
    }

  WaitFileScanTaskFinished(handle);
  return GetFileLineCount(handle) < 3 ? -2 : 0;
  }

#ifdef FJIFFYLDG_SMOKE_MAIN
int main(int argc, char** argv) {
  if (argc != 2) {
    std::fprintf(stderr, "usage: cpp_smoke <input-file>\n");
    return 64;
    }

  return fjiffyldg_cpp_header_smoke(argv[1]);
  }
#endif
