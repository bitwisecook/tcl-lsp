#pragma once

#include "tcl_lsp/core/source_position.hpp"

#include <compare>
#include <cstddef>

namespace tcl_lsp {

// A span in source text defined by start and end positions.
struct Range {
    SourcePosition start;
    SourcePosition end;

    static constexpr auto zero() noexcept -> Range {
        return {{0, 0, 0}, {0, 0, 0}};
    }

    auto operator<=>(const Range&) const = default;

    struct Hash {
        std::size_t operator()(const Range& r) const noexcept;
    };
};

auto to_string(const Range& r) -> std::string;

}  // namespace tcl_lsp
