// Core analyser entry points and command iteration.
//
// Split into three files for readability:
//   analyser.cpp          — entry points, body/command iteration
//   analyser_commands.cpp — command-specific handlers
//   analyser_helpers.cpp  — variable tracking, scope, naming, diagnostics, expr

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/stub_parser.hpp"
#include "tcl_lsp/parsing/recovery.hpp"

#include <algorithm>
#include <cctype>

namespace tcl_lsp {

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

Analyser::Analyser(const CommandRegistryInterface* registry,
                   std::unordered_set<std::string> disabled_diagnostics)
    : registry_(registry), disabled_diagnostics_(std::move(disabled_diagnostics)) {}

// ---------------------------------------------------------------------------
// Full analysis from source text
// ---------------------------------------------------------------------------

auto Analyser::analyse(std::string_view source) -> AnalysisResult {
    s_ = Session{};
    s_.current_scope = &s_.result.global_scope();

    // Pre-scan for inline stub blocks.
    auto stubs = scan_source_for_stubs(source);
    s_.result.stub_commands_.insert(s_.result.stub_commands_.end(),
                                   std::make_move_iterator(stubs.commands.begin()),
                                   std::make_move_iterator(stubs.commands.end()));
    s_.result.stub_expr_defs_.insert(s_.result.stub_expr_defs_.end(),
                                     std::make_move_iterator(stubs.expressions.begin()),
                                     std::make_move_iterator(stubs.expressions.end()));

    analyse_body(source, &s_.result.global_scope());
    emit_unresolved_command_diagnostics();
    dedupe_diagnostics();
    return std::move(s_.result);
}

// ---------------------------------------------------------------------------
// Analyse pre-segmented commands (incremental path)
// ---------------------------------------------------------------------------

auto Analyser::analyse_commands(std::string_view source,
                                const std::vector<SegmentedCommand>& commands,
                                bool finalise) -> AnalysisResult {
    s_ = Session{};
    s_.current_scope = &s_.result.global_scope();

    analyse_commands_inner(commands, &s_.result.global_scope(), source);
    if (finalise) {
        emit_unresolved_command_diagnostics();
        dedupe_diagnostics();
    }
    return std::move(s_.result);
}

// ---------------------------------------------------------------------------
// Analyse a Tcl body (top-level or nested brace body)
// ---------------------------------------------------------------------------

void Analyser::analyse_body(std::string_view source, Scope* scope,
                            const Token* body_token) {
    auto [commands, recovery_diags] = segment_with_recovery(source, body_token);
    s_.result.diagnostics_.insert(s_.result.diagnostics_.end(),
                               std::make_move_iterator(recovery_diags.begin()),
                               std::make_move_iterator(recovery_diags.end()));
    analyse_commands_inner(commands, scope, source);
}

// ---------------------------------------------------------------------------
// Inner command iteration loop
// ---------------------------------------------------------------------------

void Analyser::analyse_commands_inner(const std::vector<SegmentedCommand>& commands,
                                      Scope* scope, std::string_view /*source*/) {
    for (const auto& cmd : commands) {
        // Track preceding comments and handle noqa suppression.
        if (cmd.preceding_comment.has_value()) {
            s_.last_comment = *cmd.preceding_comment;

            auto lower = s_.last_comment;
            std::transform(lower.begin(), lower.end(), lower.begin(),
                           [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
            auto noqa_pos = lower.find("noqa");
            if (noqa_pos != std::string::npos) {
                auto rest = s_.last_comment.substr(noqa_pos + 4);
                auto ws = rest.find_first_not_of(" \t");
                if (ws != std::string::npos) {
                    rest = rest.substr(ws);
                }

                std::unordered_set<std::string> codes;
                if (!rest.empty() && rest[0] == ':') {
                    auto list = rest.substr(1);
                    std::size_t pos = 0;
                    while (pos < list.size()) {
                        auto comma = list.find(',', pos);
                        auto token = list.substr(
                            pos, comma == std::string::npos ? std::string::npos : comma - pos);
                        auto first = token.find_first_not_of(" \t");
                        auto last = token.find_last_not_of(" \t");
                        if (first != std::string::npos) {
                            codes.emplace(token.substr(first, last - first + 1));
                        }
                        if (comma == std::string::npos) break;
                        pos = comma + 1;
                    }
                } else {
                    codes.insert("*"); // suppress all
                }

                for (auto ln = cmd.range.start.line; ln <= cmd.range.end.line; ++ln) {
                    auto& existing = s_.result.suppressed_lines_[ln];
                    existing.insert(codes.begin(), codes.end());
                }
            }
        }

        // Partial (error) commands.
        if (cmd.is_partial) {
            std::string msg = "missing close-brace";
            if (cmd.partial_delimiter.has_value()) {
                switch (*cmd.partial_delimiter) {
                    case UnclosedDelimiter::BRACKET: msg = "missing close-bracket"; break;
                    case UnclosedDelimiter::QUOTE: msg = "missing \""; break;
                    case UnclosedDelimiter::BRACE: break;
                }
            }
            emit_diagnostic(cmd.range, Severity::ERROR, DiagCode::E200, msg);
            continue;
        }

        // Scan all tokens for variable reads and command substitutions.
        scan_var_references(cmd.all_tokens, scope);

        // Dispatch the command.
        process_command(cmd, scope, "");
    }
}

// ---------------------------------------------------------------------------
// Snapshot / Restore for incremental analysis
// ---------------------------------------------------------------------------

auto Analyser::snapshot() -> AnalyserSnapshot {
    return AnalyserSnapshot{
        s_.result.copy_for_snapshot(),
        s_.last_comment,
        s_.conditional_depth,
        s_.alias_resolver.aliases(),
    };
}

void Analyser::restore(AnalyserSnapshot snap) {
    s_ = Session{};
    s_.result = std::move(snap.result);
    s_.last_comment = std::move(snap.last_comment);
    s_.conditional_depth = snap.conditional_depth;
    s_.alias_resolver.set_aliases(std::move(snap.alias_state));
    s_.current_scope = &s_.result.global_scope();
}

// ---------------------------------------------------------------------------
// Chunked analysis with per-chunk snapshots
// ---------------------------------------------------------------------------

auto Analyser::analyse_chunked(std::string_view source,
                               const std::vector<std::vector<SegmentedCommand>>& chunks)
    -> std::pair<AnalysisResult, std::vector<AnalyserSnapshot>> {
    // If not restored, start fresh. If restored, continue from restored state.
    if (s_.current_scope == nullptr) {
        s_ = Session{};
        s_.current_scope = &s_.result.global_scope();
    }

    std::vector<AnalyserSnapshot> snapshots;
    snapshots.reserve(chunks.size());

    for (const auto& chunk : chunks) {
        analyse_commands_inner(chunk, &s_.result.global_scope(), source);
        snapshots.push_back(snapshot());
    }

    emit_unresolved_command_diagnostics();
    dedupe_diagnostics();
    return {std::move(s_.result), std::move(snapshots)};
}

// ---------------------------------------------------------------------------
// Convenience free function
// ---------------------------------------------------------------------------

auto analyse(std::string_view source,
             const CommandRegistryInterface* registry) -> AnalysisResult {
    return Analyser(registry).analyse(source);
}

} // namespace tcl_lsp
