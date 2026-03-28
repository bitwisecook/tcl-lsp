#pragma once

#include "tcl_lsp/analysis/auxiliary_types.hpp"
#include "tcl_lsp/core/range.hpp"

#include <string_view>
#include <utility>
#include <vector>

namespace tcl_lsp {

struct StubScanResult {
    std::vector<StubCommandDef> commands;
    std::vector<StubExprDef> expressions;
};

// Pre-scan source text for inline stub blocks (# tcl-lsp: stubs-begin/end).
[[nodiscard]] auto scan_source_for_stubs(std::string_view source) -> StubScanResult;

// Parse a single command stub line.
[[nodiscard]] auto parse_stub_line(std::string_view line, Range line_range)
    -> std::optional<StubCommandDef>;

// Parse a single expr function/operator stub line.
[[nodiscard]] auto parse_expr_stub_line(std::string_view line, Range line_range)
    -> std::optional<StubExprDef>;

// Check for stubs-begin/end markers.
[[nodiscard]] auto is_stubs_begin(std::string_view comment) -> bool;
[[nodiscard]] auto is_stubs_end(std::string_view comment) -> bool;

} // namespace tcl_lsp
