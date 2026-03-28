#pragma once

#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

namespace tcl_lsp {

// Single-pass Tcl analyser.
//
// Walks segmented commands, builds a semantic model of the source: scopes,
// proc definitions, variable definitions, and emits diagnostics.
class Analyser {
  public:
    explicit Analyser(const CommandRegistryInterface* registry = nullptr,
                      std::unordered_set<std::string> disabled_diagnostics = {});

    // Main entry point: analyse source text from scratch.
    auto analyse(std::string_view source) -> AnalysisResult;

    // Analyse pre-segmented commands (incremental path).
    auto analyse_commands(std::string_view source,
                          const std::vector<SegmentedCommand>& commands,
                          bool finalise = true) -> AnalysisResult;

  private:
    AnalysisResult result_;
    Scope* current_scope_ = nullptr;
    const CommandRegistryInterface* registry_;
    std::unordered_set<std::string> disabled_diagnostics_;
    std::string last_comment_;
    int32_t conditional_depth_ = 0;

    // Constant string tracking per scope (for regex variable propagation).
    // scope pointer -> (var_name -> (value, value_range))
    std::unordered_map<Scope*, std::unordered_map<std::string, std::pair<std::string, Range>>>
        const_strings_;

    // Variables known to hold regex patterns: (scope, var_name).
    std::unordered_set<std::string> regex_var_keys_;

    // Command aliases: qualified alias_name -> (target_cmd, prepended_args).
    std::unordered_map<std::string, std::pair<std::string, std::vector<std::string>>>
        command_aliases_;

    // Track whether we've already emitted W123 diagnostics.
    bool unresolved_commands_emitted_ = false;

    // Namespace cache: scope ptr -> qualified namespace string.
    std::unordered_map<Scope*, std::string> ns_cache_;

    // --- Analysis methods ---
    void analyse_body(std::string_view source, Scope* scope, const Token* body_token = nullptr);
    void analyse_commands_inner(const std::vector<SegmentedCommand>& commands,
                                Scope* scope, std::string_view source);
    void process_command(const SegmentedCommand& cmd, Scope* scope, std::string_view source);

    // --- Command handlers ---
    void handle_proc(const SegmentedCommand& cmd, Scope* scope);
    void handle_set(const SegmentedCommand& cmd, Scope* scope);
    void handle_namespace_eval(const SegmentedCommand& cmd, Scope* scope);
    void handle_foreach(const SegmentedCommand& cmd, Scope* scope);
    void handle_for(const SegmentedCommand& cmd, Scope* scope);
    void handle_switch(const SegmentedCommand& cmd, Scope* scope);
    void handle_catch(const SegmentedCommand& cmd, Scope* scope);
    void handle_try(const SegmentedCommand& cmd, Scope* scope);
    void handle_incr(const SegmentedCommand& cmd, Scope* scope);
    void handle_variable_decl(const SegmentedCommand& cmd, Scope* scope);
    void handle_interp_alias(const SegmentedCommand& cmd);
    void handle_package(const SegmentedCommand& cmd);
    void handle_source(const SegmentedCommand& cmd);

    // --- Generic body analysis via arg roles ---
    void analyse_body_args(const SegmentedCommand& cmd, Scope* scope);

    // --- Variable tracking ---
    void define_var(const std::string& name, Range range, Scope* scope,
                    bool warn_if_unused = false);
    void record_var_read(const std::string& name, Range range, Scope* scope);

    // --- Const string / regex tracking ---
    void set_const_string(const std::string& var_name, const std::string& value,
                          Range value_range, Scope* scope);
    void clear_const_string(const std::string& var_name, Scope* scope);
    auto lookup_const_string(const std::string& var_name, Scope* scope) const
        -> std::optional<std::pair<std::string, Range>>;
    void record_defining_set_as_regex(const std::string& var_name, Scope* scope,
                                      const std::string& command);
    auto regex_var_key(Scope* scope, const std::string& name) const -> std::string;

    // --- Scope helpers ---
    auto make_child_scope(ScopeKind kind, const std::string& name, Scope* parent) -> Scope*;
    auto namespace_from_scope(Scope* scope) -> std::string;
    auto normalise_qualified_name(const std::string& name, Scope* scope) -> std::string;
    auto normalise_var_name(const std::string& raw) -> std::string;

    // --- Unknown handler analysis ---
    void extract_unknown_proc_info(const std::string& proc_body);

    // --- Diagnostics ---
    void emit_unresolved_command_diagnostics();
    void dedupe_diagnostics();
    auto is_diagnostic_suppressed(int32_t line, const std::string& code) const -> bool;
    void emit_diagnostic(Range range, Severity severity, const std::string& code,
                         const std::string& message, std::vector<CodeFix> fixes = {});

    // --- Expression analysis ---
    void analyse_expr(std::string_view expr_text, Scope* scope);

    // --- Scan for variable references in token stream ---
    void scan_var_references(const std::vector<Token>& tokens, Scope* scope);
};

// Convenience wrapper.
auto analyse(std::string_view source,
             const CommandRegistryInterface* registry = nullptr) -> AnalysisResult;

} // namespace tcl_lsp
