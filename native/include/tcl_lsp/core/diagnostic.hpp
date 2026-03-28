#pragma once

#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <string>
#include <vector>

namespace tcl_lsp {

// Diagnostic severity levels matching LSP DiagnosticSeverity.
enum class Severity : std::uint8_t { ERROR = 1, WARNING = 2, INFORMATION = 3, HINT = 4 };

// Type-safe diagnostic codes.  Numeric values encode the category:
// 1xxx = errors, 2xxx = warnings/hints.
enum class DiagCode : std::uint16_t {
    E001 = 1001,  // builtin arity error
    E002 = 1002,  // proc call: too few arguments
    E003 = 1003,  // proc call: too many arguments
    E101 = 1101,  // missing open brace (from Python/pybind11)
    E200 = 1200,  // missing close delimiter (generic)
    E201 = 1201,  // unterminated bracket
    E202 = 1202,  // unterminated quote
    E203 = 1203,  // unterminated brace
    E204 = 1204,  // recovery: bracket
    E205 = 1205,  // recovery: quote
    E206 = 1206,  // recovery: brace
    W113 = 2113,  // proc shadows builtin
    W123 = 2123,  // unresolved command
    W214 = 2214,  // unused proc parameter
};

auto to_string(DiagCode c) -> std::string;

// A quick-fix suggestion attached to a diagnostic.
struct CodeFix {
    Range range;
    std::string new_text;
    std::string description;

    auto operator==(const CodeFix&) const -> bool = default;
};

// A diagnostic message with optional quick-fixes.
struct Diagnostic {
    Range range;
    Severity severity = Severity::ERROR;
    DiagCode code = DiagCode::E200;
    std::string message;
    std::vector<CodeFix> fixes;

    auto operator==(const Diagnostic&) const -> bool = default;
};

auto to_string(Severity s) -> std::string;

} // namespace tcl_lsp
