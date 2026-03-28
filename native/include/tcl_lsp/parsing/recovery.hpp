#pragma once

#include "tcl_lsp/core/diagnostic.hpp"
#include "tcl_lsp/core/range.hpp"
#include "tcl_lsp/core/source_position.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstdint>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

namespace tcl_lsp {

// A zero-width character the lexer should see at a source offset.
struct VirtualToken {
    int32_t offset;
    char ch;
    Diagnostic diagnostic;
};

// Result of segment_with_recovery(): clean commands + diagnostics.
struct RecoveryResult {
    std::vector<SegmentedCommand> commands;
    std::vector<Diagnostic> diagnostics;
};

// Compute absolute SourcePosition for a relative offset within text.
// O(rel_offset) — walks characters counting newlines.
[[nodiscard]] auto position_from_relative(std::string_view text,
                                          int32_t rel_offset,
                                          int32_t base_line,
                                          int32_t base_col,
                                          int32_t base_offset) -> SourcePosition;

// E201 detectors (unterminated [).
[[nodiscard]] auto detect_missing_bracket_at_comment(
    const Token& tok, std::string_view source, int32_t base_offset) -> std::optional<VirtualToken>;

[[nodiscard]] auto detect_missing_bracket_at_command(
    const Token& tok,
    std::string_view source,
    int32_t base_offset,
    const std::unordered_set<std::string>& known_commands) -> std::optional<VirtualToken>;

[[nodiscard]] auto detect_missing_bracket_at_brace(
    const Token& tok, std::string_view source, int32_t base_offset) -> std::optional<VirtualToken>;

[[nodiscard]] auto detect_missing_bracket_no_heuristic(const Token& tok) -> Diagnostic;

// E202 detectors (unterminated ").
[[nodiscard]] auto is_suspicious_quote(const Token& tok,
                                       const SegmentedCommand& cmd,
                                       std::string_view source,
                                       int32_t base_offset) -> bool;

[[nodiscard]] auto detect_missing_quote_at_newline(
    const Token& tok,
    std::string_view source,
    int32_t base_offset,
    const std::unordered_set<std::string>& known_commands) -> std::optional<VirtualToken>;

[[nodiscard]] auto detect_missing_quote_no_heuristic(const Token& tok,
                                                     std::string_view source,
                                                     int32_t base_offset) -> Diagnostic;

// E203 detectors (unterminated {).
[[nodiscard]] auto
is_suspicious_str(const Token& tok, std::string_view source, int32_t base_offset) -> bool;

[[nodiscard]] auto detect_missing_brace_at_command(
    const Token& tok,
    std::string_view source,
    int32_t base_offset,
    const std::unordered_set<std::string>& known_commands) -> std::optional<VirtualToken>;

[[nodiscard]] auto detect_missing_brace_no_heuristic(const Token& tok,
                                                     std::string_view source,
                                                     int32_t base_offset) -> Diagnostic;

// Check if a CMD token is missing its closing ].
[[nodiscard]] auto
is_unterminated_cmd(const Token& tok, std::string_view source, int32_t base_offset) -> bool;

// Compute virtual token insertions for error recovery.
// Does a first parse, detects imbalances, and returns the insertions dict.
// When known_commands is provided, command-break heuristics are enabled.
[[nodiscard]] auto compute_virtual_insertions(
    std::string_view source,
    const Token* body_token = nullptr,
    const std::unordered_set<std::string>* known_commands = nullptr)
    -> std::unordered_map<int32_t, char>;

// Parse source, detect imbalances, re-parse with virtual tokens.
// Returns (clean_commands, diagnostics).
// When known_commands is provided, command-break heuristics are enabled.
[[nodiscard]] auto segment_with_recovery(
    std::string_view source,
    const Token* body_token = nullptr,
    const std::unordered_set<std::string>* known_commands = nullptr) -> RecoveryResult;

} // namespace tcl_lsp
