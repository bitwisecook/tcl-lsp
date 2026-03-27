#pragma once

#include "tcl_lsp/core/source_position.hpp"

#include <compare>
#include <cstdint>
#include <string>
#include <string_view>

namespace tcl_lsp {

// Types of tokens produced by the Tcl lexer.
enum class TokenType : uint8_t {
    ESC,     // plain string fragment (possibly with escape sequences)
    STR,     // braced string {…}
    CMD,     // command substitution [… ]
    VAR,     // variable substitution $name
    SEP,     // whitespace separator
    EOL,     // end-of-line / semicolon
    EOF_,    // NOLINT(readability-identifier-naming) — avoid macro conflict with EOF
    COMMENT, // comment (# to end of line)
    EXPAND,  // {*} expansion prefix
};

auto to_string(TokenType type) -> std::string;

// A token with its type, text, and source location.
struct Token {
    TokenType type;
    std::string text; // token text (owned copy for Python boundary safety)
    SourcePosition start;
    SourcePosition end;
    bool in_quote = false;

    auto operator==(const Token&) const -> bool = default;
};

auto to_string(const Token& t) -> std::string;

} // namespace tcl_lsp
