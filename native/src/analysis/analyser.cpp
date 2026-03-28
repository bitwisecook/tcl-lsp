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
    result_ = AnalysisResult{};
    current_scope_ = &result_.global_scope();
    conditional_depth_ = 0;
    last_comment_.clear();
    command_aliases_.clear();
    unresolved_commands_emitted_ = false;
    ns_cache_.clear();

    // Pre-scan for inline stub blocks.
    auto stubs = scan_source_for_stubs(source);
    result_.stub_commands.insert(result_.stub_commands.end(),
                                 std::make_move_iterator(stubs.commands.begin()),
                                 std::make_move_iterator(stubs.commands.end()));
    result_.stub_expr_defs.insert(result_.stub_expr_defs.end(),
                                   std::make_move_iterator(stubs.expressions.begin()),
                                   std::make_move_iterator(stubs.expressions.end()));

    analyse_body(source, &result_.global_scope());
    emit_unresolved_command_diagnostics();
    dedupe_diagnostics();
    return std::move(result_);
}

// ---------------------------------------------------------------------------
// Analyse pre-segmented commands (incremental path)
// ---------------------------------------------------------------------------

auto Analyser::analyse_commands(std::string_view source,
                                const std::vector<SegmentedCommand>& commands,
                                bool finalise) -> AnalysisResult {
    result_ = AnalysisResult{};
    current_scope_ = &result_.global_scope();
    conditional_depth_ = 0;
    last_comment_.clear();
    command_aliases_.clear();
    unresolved_commands_emitted_ = false;
    ns_cache_.clear();

    analyse_commands_inner(commands, &result_.global_scope(), source);
    if (finalise) {
        emit_unresolved_command_diagnostics();
        dedupe_diagnostics();
    }
    return std::move(result_);
}

// ---------------------------------------------------------------------------
// Analyse a Tcl body (top-level or nested brace body)
// ---------------------------------------------------------------------------

void Analyser::analyse_body(std::string_view source, Scope* scope,
                            const Token* body_token) {
    auto [commands, recovery_diags] = segment_with_recovery(source, body_token);
    result_.diagnostics.insert(result_.diagnostics.end(),
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
            last_comment_ = *cmd.preceding_comment;

            auto lower = last_comment_;
            std::transform(lower.begin(), lower.end(), lower.begin(),
                           [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
            auto noqa_pos = lower.find("noqa");
            if (noqa_pos != std::string::npos) {
                auto rest = last_comment_.substr(noqa_pos + 4);
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
                    auto& existing = result_.suppressed_lines[ln];
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
            emit_diagnostic(cmd.range, Severity::ERROR, "E200", msg);
            continue;
        }

        // Scan all tokens for variable reads and command substitutions.
        scan_var_references(cmd.all_tokens, scope);

        // Dispatch the command.
        process_command(cmd, scope, "");
    }
}

// ---------------------------------------------------------------------------
// Convenience free function
// ---------------------------------------------------------------------------

auto analyse(std::string_view source,
             const CommandRegistryInterface* registry) -> AnalysisResult {
    return Analyser(registry).analyse(source);
}

} // namespace tcl_lsp
