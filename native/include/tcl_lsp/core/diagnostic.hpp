#pragma once

#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <string>
#include <vector>

namespace tcl_lsp {

// Diagnostic severity levels matching LSP DiagnosticSeverity.
enum class Severity : std::uint8_t { ERROR = 1, WARNING = 2, INFORMATION = 3, HINT = 4 };

// A quick-fix suggestion attached to a diagnostic.
struct CodeFix {
    Range range;
    std::string new_text;
    std::string description;

    auto operator==(const CodeFix&) const -> bool = default;
};

// A diagnostic message with optional quick-fixes.
// Lightweight version for Phase 3; Phase 4 may add related_information, tags, etc.
struct Diagnostic {
    Range range;
    Severity severity = Severity::ERROR;
    std::string code;
    std::string message;
    std::vector<CodeFix> fixes;

    auto operator==(const Diagnostic&) const -> bool = default;
};

auto to_string(Severity s) -> std::string;

} // namespace tcl_lsp
