#pragma once

#include <cstddef>
#include <cstdint>

namespace tcl_lsp {

// Snapshot of C++ heap memory usage.
// On Linux, backed by mallinfo2(). On other platforms, fields are zero.
struct MemoryStats {
    std::size_t arena_bytes;     // Total bytes allocated via mmap/sbrk (non-mmap).
    std::size_t mmap_bytes;      // Bytes allocated via mmap.
    std::size_t used_bytes;      // Currently in-use bytes (arena - free).
    std::size_t free_bytes;      // Free bytes within arena.
    std::size_t total_allocated; // arena_bytes + mmap_bytes
};

// Query the C heap allocator for current memory stats.
auto memory_stats() -> MemoryStats;

}  // namespace tcl_lsp
