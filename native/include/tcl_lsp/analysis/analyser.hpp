#pragma once

#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstdint>
#include <optional>
#include <span>
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


    // Manages command alias registration and resolution.
    struct AliasResolver {
        void register_alias(const std::string& qualified,
                            const std::string& target_cmd,
                            std::vector<std::string> prepended_args);
        auto resolve(const std::string& cmd_name,
                     std::span<const std::string> args,
                     const std::string& ns) const
            -> std::pair<std::string, std::vector<std::string>>;
        void clear() { aliases_.clear(); }

      private:
        std::unordered_map<std::string, std::pair<std::string, std::vector<std::string>>>
            aliases_;
    };
    AliasResolver alias_resolver_;

    // Track whether we've already emitted W123 diagnostics.
    bool unresolved_commands_emitted_ = false;


    // --- Analysis methods ---
    void analyse_body(std::string_view source, Scope* scope, const Token* body_token = nullptr);
    void analyse_commands_inner(const std::vector<SegmentedCommand>& commands,
                                Scope* scope, std::string_view source);
    void process_command(const SegmentedCommand& cmd, Scope* scope, std::string_view source);

    // --- Command handlers (consuming: return true if command was fully handled) ---
    using ConsumingHandler = auto(Analyser::*)(const SegmentedCommand&, Scope*) -> bool;
    using NonConsumingHandler = void(Analyser::*)(const SegmentedCommand&, Scope*);

    auto handle_proc(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_namespace_eval(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_foreach(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_for(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_switch(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_catch(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_try(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_if(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_while(const SegmentedCommand& cmd, Scope* scope) -> bool;
    auto handle_dict(const SegmentedCommand& cmd, Scope* scope) -> bool;

    // Non-consuming handlers (augment the generic analysis path).
    void handle_set(const SegmentedCommand& cmd, Scope* scope);
    void handle_incr(const SegmentedCommand& cmd, Scope* scope);
    void handle_variable_decl(const SegmentedCommand& cmd, Scope* scope);
    void handle_interp_alias(const SegmentedCommand& cmd);
    void handle_package(const SegmentedCommand& cmd);
    void handle_source(const SegmentedCommand& cmd);
    void handle_expr(const SegmentedCommand& cmd, Scope* scope);

    // --- Switch body parsing ---
    void parse_switch_body(std::string_view body_text, const Token* body_token,
                           Scope* scope, bool is_regexp);

    // --- Variable list parsing (foreach) ---
    void define_vars_from_list(const std::string& var_list_text,
                               const Token& tok, Scope* scope);

    // --- Proc call ---
    auto find_proc_call(const std::string& cmd_name, Scope* scope) -> ProcDef*;
    void check_proc_call_arity(const ProcDef& proc_def,
                                std::span<const std::string> args,
                                const Token& cmd_token);

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
