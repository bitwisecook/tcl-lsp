#include "tcl_lsp/parsing/recovery.hpp"

#include <algorithm>
#include <cstddef>
#include <iterator>
#include <string>

namespace tcl_lsp {

namespace {

// Compute the base offset used by make_lexer for a body_token.
auto base_offset_for(const Token* body_token) -> int32_t {
    if (body_token == nullptr) {
        return 0;
    }
    if (body_token->type == TokenType::STR || body_token->type == TokenType::CMD) {
        return body_token->start.offset + 1;
    }
    return body_token->start.offset;
}

// Compute absolute SourcePosition for tok.text[text_idx].
// tok.start points to the '[' delimiter; inner content starts at tok.start.offset + 1.
auto cmd_text_position(const Token& tok, int32_t text_idx) -> SourcePosition {
    return position_from_relative(
        tok.text, text_idx, tok.start.line, tok.start.character + 1, tok.start.offset + 1);
}

// Extract the first word from a stripped line.
auto extract_first_word(std::string_view stripped) -> std::string_view {
    std::size_t end = 0;
    while (end < stripped.size()) {
        const char ch = stripped[end];
        if (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == ';' || ch == '{' ||
            ch == '[') {
            break;
        }
        ++end;
    }
    return stripped.substr(0, end);
}

// Split text by newlines, returning views into the original text.
auto split_lines(std::string_view text) -> std::vector<std::string_view> {
    std::vector<std::string_view> lines;
    std::size_t start = 0;
    while (start <= text.size()) {
        auto nl = text.find('\n', start);
        if (nl == std::string_view::npos) {
            lines.push_back(text.substr(start));
            break;
        }
        lines.push_back(text.substr(start, nl - start));
        start = nl + 1;
    }
    return lines;
}

// Count leading whitespace characters.
auto leading_whitespace(std::string_view line) -> std::size_t {
    std::size_t n = 0;
    while (n < line.size() && (line[n] == ' ' || line[n] == '\t')) {
        ++n;
    }
    return n;
}

// Trim trailing whitespace from a string_view.
auto rtrim(std::string_view sv) -> std::string_view {
    while (!sv.empty() && (sv.back() == ' ' || sv.back() == '\t' || sv.back() == '\r')) {
        sv.remove_suffix(1);
    }
    return sv;
}

// Detect all ghost tokens across all commands.
// When known_commands is non-null, command-break heuristics are enabled.
auto detect_all_ghost_tokens(const std::vector<SegmentedCommand>& commands,
                             std::string_view source,
                             int32_t base_offset,
                             const std::unordered_set<std::string>* known_commands)
    -> std::pair<std::vector<GhostToken>, std::vector<Diagnostic>> {

    std::vector<GhostToken> ghosts;
    std::vector<Diagnostic> fallback_diags;

    // Empty set used when caller provides no known commands.
    static const std::unordered_set<std::string> empty_set;
    const auto& known_cmds =
        (known_commands != nullptr && !known_commands->empty()) ? *known_commands : empty_set;

    for (const auto& cmd : commands) {
        for (const auto& tok : cmd.all_tokens) {
            // E201: unterminated [
            if (is_unterminated_cmd(tok, source, base_offset)) {
                auto vt = detect_missing_bracket_at_comment(tok, source, base_offset);
                if (!vt) {
                    vt = detect_missing_bracket_at_command(tok, source, base_offset, known_cmds);
                }
                if (!vt) {
                    vt = detect_missing_bracket_at_brace(tok, source, base_offset);
                }
                if (vt) {
                    ghosts.push_back(std::move(*vt));
                } else {
                    fallback_diags.push_back(detect_missing_bracket_no_heuristic(tok));
                }
                continue;
            }

            // E202: unterminated "
            if (is_suspicious_quote(tok, cmd, source, base_offset)) {
                auto vt = detect_missing_quote_at_newline(tok, source, base_offset, known_cmds);
                if (vt) {
                    ghosts.push_back(std::move(*vt));
                } else {
                    fallback_diags.push_back(
                        detect_missing_quote_no_heuristic(tok, source, base_offset));
                }
                continue;
            }

            // E203: unterminated {
            if (is_suspicious_str(tok, source, base_offset)) {
                auto vt = detect_missing_brace_at_command(tok, source, base_offset, known_cmds);
                if (vt) {
                    ghosts.push_back(std::move(*vt));
                } else {
                    fallback_diags.push_back(
                        detect_missing_brace_no_heuristic(tok, source, base_offset));
                }
            }
        }
    }

    return {std::move(ghosts), std::move(fallback_diags)};
}

} // anonymous namespace

// position_from_relative: O(rel_offset) walk counting newlines.
auto position_from_relative(std::string_view text,
                            int32_t rel_offset,
                            int32_t base_line,
                            int32_t base_col,
                            int32_t base_offset) -> SourcePosition {
    auto rel = std::clamp(rel_offset, static_cast<int32_t>(0), static_cast<int32_t>(text.size()));
    int32_t line = base_line;
    int32_t col = base_col;
    for (int32_t i = 0; i < rel; ++i) {
        if (text[static_cast<std::size_t>(i)] == '\n') {
            ++line;
            col = 0;
        } else {
            ++col;
        }
    }
    return {line, col, base_offset + rel};
}

// E201: unterminated [ — comment-break heuristic.
auto detect_missing_bracket_at_comment(const Token& tok,
                                       std::string_view /*source*/,
                                       int32_t base_offset) -> std::optional<GhostToken> {

    auto lines = split_lines(tok.text);
    if (lines.size() < 2) {
        return std::nullopt;
    }

    for (std::size_t i = 0; i < lines.size(); ++i) {
        if (i == 0) {
            continue;
        }
        auto lw = leading_whitespace(lines[i]);
        auto stripped = lines[i].substr(lw);

        if (!stripped.empty() && stripped.front() == '#') {
            // Found a comment. Insert ] at end of previous line.
            auto prev_line = lines[i - 1];
            auto content_end = static_cast<int32_t>(rtrim(prev_line).size());
            int32_t insert_text_idx = 0;
            if (i == 1) {
                insert_text_idx = content_end;
            } else {
                insert_text_idx = 0;
                for (std::size_t j = 0; j < i - 1; ++j) {
                    insert_text_idx += static_cast<int32_t>(lines[j].size()) + 1;
                }
                insert_text_idx += content_end;
            }

            const int32_t local_bracket_start = tok.start.offset - base_offset;
            const int32_t ghost_offset = local_bracket_start + 1 + insert_text_idx;

            auto diag_end = cmd_text_position(tok, std::max(insert_text_idx - 1, 0));
            const Range diag_range{tok.start, diag_end};

            auto insert_pos = cmd_text_position(tok, insert_text_idx);
            const Range fix_range{insert_pos, insert_pos};

            return GhostToken{
                ghost_offset,
                ']',
                Diagnostic{
                    diag_range,
                    Severity::ERROR,
                    "E201",
                    "missing close-bracket",
                    {CodeFix{fix_range, "]", "Insert missing ']' before comment"}},
                },
            };
        }
        // Non-empty, non-comment line — stop looking.
        if (!stripped.empty()) {
            break;
        }
    }

    return std::nullopt;
}

// E201: unterminated [ — command-break heuristic.
auto detect_missing_bracket_at_command(const Token& tok,
                                       std::string_view /*source*/,
                                       int32_t base_offset,
                                       const std::unordered_set<std::string>& known_commands)
    -> std::optional<GhostToken> {

    auto lines = split_lines(tok.text);
    if (lines.size() < 2) {
        return std::nullopt;
    }

    for (std::size_t i = 1; i < lines.size(); ++i) {
        auto lw = leading_whitespace(lines[i]);
        auto stripped = lines[i].substr(lw);
        if (stripped.empty()) {
            continue; // skip blank lines
        }
        auto first_word = extract_first_word(stripped);
        if (known_commands.contains(std::string(first_word))) {
            auto prev_line = lines[i - 1];
            auto content_end = static_cast<int32_t>(rtrim(prev_line).size());
            int32_t insert_text_idx = 0;
            if (i == 1) {
                insert_text_idx = content_end;
            } else {
                insert_text_idx = 0;
                for (std::size_t j = 0; j < i - 1; ++j) {
                    insert_text_idx += static_cast<int32_t>(lines[j].size()) + 1;
                }
                insert_text_idx += content_end;
            }

            const int32_t local_bracket_start = tok.start.offset - base_offset;
            const int32_t ghost_offset = local_bracket_start + 1 + insert_text_idx;

            auto diag_end = cmd_text_position(tok, std::max(insert_text_idx - 1, 0));
            const Range diag_range{tok.start, diag_end};

            auto insert_pos = cmd_text_position(tok, insert_text_idx);
            const Range fix_range{insert_pos, insert_pos};

            return GhostToken{
                ghost_offset,
                ']',
                Diagnostic{
                    diag_range,
                    Severity::ERROR,
                    "E201",
                    "missing close-bracket",
                    {CodeFix{fix_range, "]", "Insert missing ']' before command"}},
                },
            };
        }
        // Non-empty, non-command line — stop looking.
        break;
    }

    return std::nullopt;
}

// E201: unterminated [ — brace-break heuristic.
auto detect_missing_bracket_at_brace(const Token& tok,
                                     std::string_view /*source*/,
                                     int32_t base_offset) -> std::optional<GhostToken> {

    auto brace_pos = tok.text.find('{');
    if (brace_pos == std::string::npos) {
        return std::nullopt;
    }

    // Step back past whitespace before the brace.
    auto insert_idx = static_cast<int32_t>(brace_pos);
    while (insert_idx > 0 && (tok.text[static_cast<std::size_t>(insert_idx - 1)] == ' ' ||
                              tok.text[static_cast<std::size_t>(insert_idx - 1)] == '\t')) {
        --insert_idx;
    }

    // Check that there's actual content before the brace.
    auto content =
        rtrim(std::string_view(tok.text).substr(0, static_cast<std::size_t>(insert_idx)));
    if (content.empty()) {
        return std::nullopt;
    }

    const int32_t local_bracket_start = tok.start.offset - base_offset;
    const int32_t ghost_offset = local_bracket_start + 1 + insert_idx;

    auto content_end_idx = std::max(static_cast<int32_t>(content.size()) - 1, 0);
    auto diag_end = cmd_text_position(tok, content_end_idx);
    const Range diag_range{tok.start, diag_end};

    auto insert_pos = cmd_text_position(tok, insert_idx);
    const Range fix_range{insert_pos, insert_pos};

    return GhostToken{
        ghost_offset,
        ']',
        Diagnostic{
            diag_range,
            Severity::ERROR,
            "E201",
            "missing close-bracket",
            {CodeFix{fix_range, "]", "Insert missing ']' before '{'"}},
        },
    };
}

// E201: fallback when no heuristic matches.
auto detect_missing_bracket_no_heuristic(const Token& tok) -> Diagnostic {
    return Diagnostic{
        Range{tok.start, tok.start},
        Severity::ERROR,
        "E201",
        "missing close-bracket",
        {},
    };
}

// E202: check if an ESC token is from an unterminated " at EOL.
auto is_suspicious_quote(const Token& tok,
                         const SegmentedCommand& cmd,
                         std::string_view source,
                         int32_t base_offset) -> bool {
    if (tok.type != TokenType::ESC) {
        return false;
    }
    auto src_idx = tok.start.offset - base_offset;
    if (src_idx < 0 || static_cast<std::size_t>(src_idx) >= source.size() ||
        source[static_cast<std::size_t>(src_idx)] != '"') {
        return false;
    }
    if (tok.text.empty() || tok.text[0] != '\n') {
        return false;
    }
    // Check there's content after the newline.
    auto rest = std::string_view(tok.text).substr(1);
    const bool has_content = std::any_of(rest.begin(), rest.end(), [](char ch) {
        return ch != ' ' && ch != '\t' && ch != '\n' && ch != '\r';
    });
    if (!has_content) {
        return false;
    }
    // The command must reach EOF.
    if (!cmd.all_tokens.empty()) {
        const auto& last = cmd.all_tokens.back();
        if (last.end.offset < base_offset + static_cast<int32_t>(source.size()) - 1) {
            return false;
        }
    }
    return true;
}

// E202: newline heuristic.
auto detect_missing_quote_at_newline(const Token& tok,
                                     std::string_view /*source*/,
                                     int32_t base_offset,
                                     const std::unordered_set<std::string>& known_commands)
    -> std::optional<GhostToken> {

    auto lines = split_lines(tok.text);
    if (lines.size() < 2) {
        return std::nullopt;
    }

    for (std::size_t i = 1; i < lines.size(); ++i) {
        auto lw = leading_whitespace(lines[i]);
        auto stripped = lines[i].substr(lw);
        if (stripped.empty()) {
            continue; // skip blank lines
        }
        auto first_word = extract_first_word(stripped);
        if (known_commands.contains(std::string(first_word))) {
            // Virtual " right after the opening " -> creates empty string "".
            const int32_t local_quote_start = tok.start.offset - base_offset;
            const int32_t ghost_offset = local_quote_start + 1;

            const Range diag_range{tok.start, tok.start};

            const SourcePosition insert_pos{
                tok.start.line, tok.start.character + 1, tok.start.offset + 1};
            const Range fix_range{insert_pos, insert_pos};

            return GhostToken{
                ghost_offset,
                '"',
                Diagnostic{
                    diag_range,
                    Severity::ERROR,
                    "E202",
                    "missing \"",
                    {CodeFix{fix_range, "\"", "Insert missing '\"' to close string"}},
                },
            };
        }
        // First non-blank line is not a known command — stop.
        break;
    }

    return std::nullopt;
}

// E202: fallback.
auto detect_missing_quote_no_heuristic(const Token& tok,
                                       std::string_view /*source*/,
                                       int32_t /*base_offset*/) -> Diagnostic {
    return Diagnostic{
        Range{tok.start, tok.start},
        Severity::ERROR,
        "E202",
        "missing \"",
        {},
    };
}

// E203: check if a STR token is from an unterminated {.
auto is_suspicious_str(const Token& tok, std::string_view source, int32_t base_offset) -> bool {
    if (tok.type != TokenType::STR) {
        return false;
    }
    auto src_idx = tok.start.offset - base_offset;
    if (src_idx < 0 || static_cast<std::size_t>(src_idx) >= source.size() ||
        source[static_cast<std::size_t>(src_idx)] != '{') {
        return false;
    }
    // Check closing } is missing.
    auto close_local = tok.end.offset - base_offset + 1;
    if (close_local >= 0 && static_cast<std::size_t>(close_local) < source.size() &&
        source[static_cast<std::size_t>(close_local)] == '}') {
        return false; // properly closed
    }
    // If text contains }, it's E103 territory (stolen close brace), not E203.
    if (tok.text.find('}') != std::string::npos) {
        return false;
    }
    // Must span multiple lines.
    const int32_t line_span = tok.end.line - tok.start.line;
    return line_span >= 2;
}

// E203: de-indented command heuristic.
auto detect_missing_brace_at_command(const Token& tok,
                                     std::string_view /*source*/,
                                     int32_t base_offset,
                                     const std::unordered_set<std::string>& known_commands)
    -> std::optional<GhostToken> {

    auto lines = split_lines(tok.text);
    if (lines.size() < 3) {
        return std::nullopt;
    }

    // Determine indentation of the first content line.
    int32_t first_indent = -1;
    for (std::size_t i = 1; i < lines.size(); ++i) {
        auto lw = leading_whitespace(lines[i]);
        auto stripped = lines[i].substr(lw);
        if (!stripped.empty()) {
            first_indent = static_cast<int32_t>(lw);
            break;
        }
    }
    if (first_indent < 0) {
        return std::nullopt;
    }

    // Scan for a de-indented line starting with a known command.
    int32_t cumulative = 0;
    for (std::size_t i = 0; i < lines.size(); ++i) {
        if (i == 0) {
            cumulative += static_cast<int32_t>(lines[i].size()) + 1;
            continue;
        }
        auto lw = leading_whitespace(lines[i]);
        auto stripped = lines[i].substr(lw);
        if (stripped.empty()) {
            cumulative += static_cast<int32_t>(lines[i].size()) + 1;
            continue;
        }
        auto indent = static_cast<int32_t>(lw);
        if (indent < first_indent) {
            auto first_word = extract_first_word(stripped);
            if (known_commands.contains(std::string(first_word))) {
                // Verify brace content before this point is balanced.
                auto content_before =
                    std::string_view(tok.text).substr(0, static_cast<std::size_t>(cumulative));
                int32_t brace_depth = 0;
                for (const char ch : content_before) {
                    if (ch == '{') {
                        ++brace_depth;
                    } else if (ch == '}') {
                        --brace_depth;
                    }
                }
                if (brace_depth != 0) {
                    cumulative += static_cast<int32_t>(lines[i].size()) + 1;
                    continue;
                }

                // Virtual } at the \n before this line.
                const int32_t newline_text_idx = cumulative - 1;
                const int32_t local_brace_start = tok.start.offset - base_offset;
                const int32_t ghost_offset = local_brace_start + 1 + newline_text_idx;

                const Range diag_range{tok.start, tok.start};

                auto insert_pos = cmd_text_position(tok, newline_text_idx);
                const Range fix_range{insert_pos, insert_pos};

                return GhostToken{
                    ghost_offset,
                    '}',
                    Diagnostic{
                        diag_range,
                        Severity::ERROR,
                        "E203",
                        "missing close-brace",
                        {CodeFix{fix_range, "}", "Insert missing '}' before command"}},
                    },
                };
            }
        }
        cumulative += static_cast<int32_t>(lines[i].size()) + 1;
    }

    return std::nullopt;
}

// E203: fallback.
auto detect_missing_brace_no_heuristic(const Token& tok,
                                       std::string_view /*source*/,
                                       int32_t /*base_offset*/) -> Diagnostic {
    return Diagnostic{
        Range{tok.start, tok.start},
        Severity::ERROR,
        "E203",
        "missing close-brace",
        {},
    };
}

// Check if a CMD token is missing its closing ].
auto is_unterminated_cmd(const Token& tok, std::string_view source, int32_t base_offset) -> bool {
    if (tok.type != TokenType::CMD) {
        return false;
    }
    auto close_local = tok.end.offset - base_offset + 1;
    return close_local < 0 || static_cast<std::size_t>(close_local) >= source.size() ||
           source[static_cast<std::size_t>(close_local)] != ']';
}

// Compute ghost insertions for error recovery.
auto compute_ghost_insertions(std::string_view source,
                              const Token* body_token,
                              const std::unordered_set<std::string>* known_commands)
    -> std::unordered_map<int32_t, char> {

    auto commands = segment_commands(source, body_token);
    if (commands.empty()) {
        return {};
    }

    const int32_t base = base_offset_for(body_token);
    auto [ghosts, _] = detect_all_ghost_tokens(commands, source, base, known_commands);

    if (ghosts.empty()) {
        return {};
    }

    std::unordered_map<int32_t, char> result;
    for (const auto& vt : ghosts) {
        result[vt.offset] = vt.ch;
    }
    return result;
}

// Full recovery pipeline.
auto segment_with_recovery(std::string_view source,
                           const Token* body_token,
                           const std::unordered_set<std::string>* known_commands)
    -> RecoveryResult {
    std::vector<std::pair<SourcePosition, std::string>> lexer_warnings;
    auto commands = segment_commands(source, body_token, nullptr, nullptr, &lexer_warnings);

    // Convert lexer warnings to diagnostics (skip recovery-handled ones).
    auto warnings_to_diags = [](const std::vector<std::pair<SourcePosition, std::string>>& warnings)
        -> std::vector<Diagnostic> {
        std::vector<Diagnostic> diags;
        for (const auto& [pos, message] : warnings) {
            // Skip warnings that overlap with recovery heuristics.
            if (message == "missing close-bracket" || message == "missing \"" ||
                message == "missing close-brace") {
                continue;
            }
            std::string code = "E200";
            if (message == "extra characters after close-brace") {
                code = "E204";
            } else if (message == "extra characters after close-quote") {
                code = "E205";
            } else if (message == "missing close-brace for variable name") {
                code = "E206";
            }
            diags.push_back(Diagnostic{Range{pos, pos}, Severity::ERROR, code, message, {}});
        }
        return diags;
    };

    if (commands.empty()) {
        return {std::move(commands), warnings_to_diags(lexer_warnings)};
    }

    const int32_t base = base_offset_for(body_token);
    auto [ghosts, fallback_diags] = detect_all_ghost_tokens(commands, source, base, known_commands);
    auto warning_diags = warnings_to_diags(lexer_warnings);

    if (ghosts.empty()) {
        auto all_diags = std::move(fallback_diags);
        all_diags.insert(all_diags.end(), warning_diags.begin(), warning_diags.end());
        return {std::move(commands), std::move(all_diags)};
    }

    // Re-parse with ghost tokens injected.
    std::unordered_map<int32_t, char> insertions;
    for (const auto& vt : ghosts) {
        insertions[vt.offset] = vt.ch;
    }
    std::vector<std::pair<SourcePosition, std::string>> reparse_warnings;
    auto clean_commands =
        segment_commands(source, body_token, nullptr, &insertions, &reparse_warnings);

    std::vector<Diagnostic> diagnostics;
    diagnostics.reserve(ghosts.size());
    std::transform(ghosts.begin(),
                   ghosts.end(),
                   std::back_inserter(diagnostics),
                   [](const GhostToken& vt) { return vt.diagnostic; });
    diagnostics.insert(diagnostics.end(), fallback_diags.begin(), fallback_diags.end());
    diagnostics.insert(diagnostics.end(), warning_diags.begin(), warning_diags.end());
    auto reparse_diags = warnings_to_diags(reparse_warnings);
    diagnostics.insert(diagnostics.end(), reparse_diags.begin(), reparse_diags.end());

    return {std::move(clean_commands), std::move(diagnostics)};
}

} // namespace tcl_lsp
