#pragma once

#include "tcl_lsp/analysis/analysis_result.hpp"

#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace tcl_lsp {

// LSP FoldingRangeKind subset.
enum class FoldingRangeKind : std::uint8_t {
    COMMENT,
    REGION,
};

// A folding range in the document.
struct FoldingRangeInfo {
    std::int32_t start_line;
    std::int32_t end_line;
    FoldingRangeKind kind;
};

// Generate folding ranges from source and optional analysis result.
[[nodiscard]] auto get_folding_ranges(std::string_view source,
                                       const AnalysisResult* analysis = nullptr)
    -> std::vector<FoldingRangeInfo>;

} // namespace tcl_lsp
