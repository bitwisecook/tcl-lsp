#include "tcl_lsp/core/range.hpp"

namespace tcl_lsp {

auto Range::Hash::operator()(const Range& r) const noexcept -> std::size_t {
    auto h = SourcePosition::Hash{};
    std::size_t seed = h(r.start);
    seed ^= h(r.end) + 0x9e3779b9 + (seed << 6) + (seed >> 2);
    return seed;
}

auto to_string(const Range& r) -> std::string {
    return "Range(start=" + to_string(r.start) + ", end=" + to_string(r.end) + ")";
}

}  // namespace tcl_lsp
