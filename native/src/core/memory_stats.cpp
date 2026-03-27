#include "tcl_lsp/core/memory_stats.hpp"

#ifdef __linux__
#include <malloc.h>
#endif

namespace tcl_lsp {

auto memory_stats() -> MemoryStats {
#ifdef __linux__
    auto info = mallinfo2();
    return {
        .arena_bytes = static_cast<std::size_t>(info.arena),
        .mmap_bytes = static_cast<std::size_t>(info.hblkhd),
        .used_bytes = static_cast<std::size_t>(info.uordblks),
        .free_bytes = static_cast<std::size_t>(info.fordblks),
        .total_allocated = static_cast<std::size_t>(info.arena)
                         + static_cast<std::size_t>(info.hblkhd),
    };
#else
    return {};
#endif
}

}  // namespace tcl_lsp
