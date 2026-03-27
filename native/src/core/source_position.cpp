#include "tcl_lsp/core/source_position.hpp"

#include <functional>

namespace tcl_lsp {

auto SourcePosition::Hash::operator()(const SourcePosition& p) const noexcept -> std::size_t {
    // Combine all three fields. offset alone is unique within a document,
    // but including line/character makes cross-document hashing more robust.
    auto h = std::hash<int32_t>{};
    std::size_t seed = h(p.offset);
    seed ^= h(p.line) + 0x9e3779b9 + (seed << 6) + (seed >> 2);
    seed ^= h(p.character) + 0x9e3779b9 + (seed << 6) + (seed >> 2);
    return seed;
}

auto to_string(const SourcePosition& p) -> std::string {
    return "SourcePosition(line=" + std::to_string(p.line)
         + ", character=" + std::to_string(p.character)
         + ", offset=" + std::to_string(p.offset) + ")";
}

}  // namespace tcl_lsp
