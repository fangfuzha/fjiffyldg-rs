#include "fjiffyldg.h"

/**
 * Compiles a minimal C++ caller against the generated public header.
 *
 * This validates the C++ convenience wrapper injected by cbindgen configuration
 * without linking against the Rust library.
 */
int fjiffyldg_cpp_header_smoke() {
    Fjiffyldg::Fjiffyldg model;
    fjiffyldg_ptr handle = model.GetFjiffyldgHandle();
    return handle == nullptr ? -1 : 0;
}
