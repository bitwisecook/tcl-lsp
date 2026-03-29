#pragma once

#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <string>
#include <vector>

namespace tcl_lsp {

// LSP SymbolKind subset used by document symbols.
enum class SymbolKind : std::int32_t {
    FUNCTION = 12,
    VARIABLE = 13,
    NAMESPACE = 3,
};

// A document symbol for the outline/breadcrumbs.
struct DocumentSymbolInfo {
    std::string name;
    std::string detail;
    SymbolKind kind;
    Range range;             // full extent
    Range selection_range;   // name only
    std::vector<DocumentSymbolInfo> children;
};

// Generate document symbols from an analysis result.
[[nodiscard]] auto get_document_symbols(const AnalysisResult& analysis)
    -> std::vector<DocumentSymbolInfo>;

} // namespace tcl_lsp
