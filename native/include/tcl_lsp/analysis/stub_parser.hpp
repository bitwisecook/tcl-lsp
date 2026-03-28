#pragma once

#include "tcl_lsp/analysis/auxiliary_types.hpp"
#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <expected>
#include <string_view>
#include <utility>
#include <vector>

namespace tcl_lsp {

// Why a stub line failed to parse.
enum class StubParseError : std::uint8_t {
    NOT_A_STUB,           // line doesn't start with "stub"
    MISSING_NAME,         // "stub" but no command name
    MISSING_BRACES,       // no {args} block found
    INVALID_ARG_SYNTAX,   // malformed arg (empty name, bad optional syntax)
    INVALID_ROLE,         // unrecognised arg role
    INVALID_EXPR_KIND,    // not "expr-func" or "expr-op"
};

struct StubScanResult {
    std::vector<StubCommandDef> commands;
    std::vector<StubExprDef> expressions;
};

// Pre-scan source text for inline stub blocks (# tcl-lsp: stubs-begin/end).
[[nodiscard]] auto scan_source_for_stubs(std::string_view source) -> StubScanResult;

// Parse a single command stub line.
[[nodiscard]] auto parse_stub_line(std::string_view line, Range line_range)
    -> std::expected<StubCommandDef, StubParseError>;

// Parse a single expr function/operator stub line.
[[nodiscard]] auto parse_expr_stub_line(std::string_view line, Range line_range)
    -> std::expected<StubExprDef, StubParseError>;

// Check for stubs-begin/end markers.
[[nodiscard]] auto is_stubs_begin(std::string_view comment) -> bool;
[[nodiscard]] auto is_stubs_end(std::string_view comment) -> bool;

} // namespace tcl_lsp
