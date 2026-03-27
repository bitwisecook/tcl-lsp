#pragma once

#include <compare>
#include <cstddef>
#include <cstdint>
#include <string>

namespace tcl_lsp {

// A position in source text (0-based line and character).
struct SourcePosition {
    int32_t line;       // 0-based line number
    int32_t character;  // 0-based column (UTF-16 code units per LSP spec)
    int32_t offset;     // byte offset into the source string

    auto operator<=>(const SourcePosition&) const = default;

    struct Hash {
        std::size_t operator()(const SourcePosition& p) const noexcept;
    };
};

auto to_string(const SourcePosition& p) -> std::string;

}  // namespace tcl_lsp
