#pragma once

#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <optional>
#include <string_view>

namespace tcl_lsp {

// Find the definition location for the symbol at the given position.
// Returns the range of the definition, or nullopt if not found.
[[nodiscard]] auto find_definition(std::string_view source,
                                    std::int32_t line,
                                    std::int32_t character,
                                    const AnalysisResult& analysis)
    -> std::optional<Range>;

// Find all references to the symbol at the given position.
[[nodiscard]] auto find_references(std::string_view source,
                                    std::int32_t line,
                                    std::int32_t character,
                                    const AnalysisResult& analysis)
    -> std::vector<Range>;

} // namespace tcl_lsp
