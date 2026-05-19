#include "fjiffyldg.h"

/**
 * Compiles a minimal C++ caller against the public header.
 *
 * This validates the C++ convenience wrapper exposed by the reference header
 * shape without linking against the Rust library.
 */
int fjiffyldg_cpp_header_smoke() {
    Fjiffyldg::Fjiffyldg model;
    fjiffyldg_ptr handle = model.GetFjiffyldgHandle();
    return handle == nullptr ? -1 : 0;
}
