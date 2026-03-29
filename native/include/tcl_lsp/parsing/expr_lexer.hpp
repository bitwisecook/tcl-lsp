#pragma once

#include <cstdint>
#include <string_view>
#include <vector>

namespace tcl_lsp {

// Token types specific to Tcl expressions.
enum class ExprTokenType : std::uint8_t {
    NUMBER,      // integer or float literal
    STRING,      // "quoted string" or {braced}
    VARIABLE,    // $var, $ns::var, $arr(idx)
    COMMAND,     // [cmd ...]
    OPERATOR,    // + - * / % ** == != < > <= >= && || ! & | ^ ~ << >>
    PAREN_OPEN,  // (
    PAREN_CLOSE, // )
    COMMA,       // ,
    FUNCTION,    // math function name: sin, cos, int, double, etc.
    BOOL,        // true, false, yes, no, on, off
    TERNARY_Q,   // ?
    TERNARY_C,   // : (ternary colon)
    WHITESPACE,  // spaces/tabs/newlines
    EOF_,
};

// A single token in a Tcl expression.
struct ExprToken {
    ExprTokenType type;
    std::string_view text;
    std::int32_t start; // offset within the expression string
    std::int32_t end;   // offset of last character

    auto operator==(const ExprToken&) const -> bool = default;
};

// Tokenise a Tcl expression string.
[[nodiscard]] auto tokenise_expr(std::string_view source) -> std::vector<ExprToken>;

} // namespace tcl_lsp
