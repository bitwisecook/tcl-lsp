#pragma once

#include "tcl_lsp/analysis/auxiliary_types.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"
#include "tcl_lsp/core/diagnostic.hpp"

#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

namespace tcl_lsp {

// Complete analysis result for a single document.
//
// Owns the scope tree via unique_ptr to the root Scope.  Child scopes are
// owned by their parent's `children` vector (stored as unique_ptr).
// Non-root scopes use raw parent pointers (safe: parent outlives children).
// Hash for (line, character) pairs used in regex position sets.
struct Int32PairHash {
    auto operator()(const std::pair<int32_t, int32_t>& p) const noexcept -> std::size_t {
        return std::hash<int64_t>{}(
            (static_cast<int64_t>(p.first) << 32) | static_cast<int64_t>(p.second));
    }
};

using RegexPositionSet = std::unordered_set<std::pair<int32_t, int32_t>, Int32PairHash>;

// Complete analysis result for a single document.
//
// Owns the scope tree via unique_ptr to the root Scope.  Child scopes are
// owned by their parent's `children` vector (stored as raw pointers).
// Non-root scopes use raw parent pointers (safe: parent outlives children).
class AnalysisResult {
  public:
    AnalysisResult();
    ~AnalysisResult();
    AnalysisResult(AnalysisResult&&) noexcept;
    auto operator=(AnalysisResult&&) noexcept -> AnalysisResult&;

    // Non-copyable (use copy_for_snapshot for explicit copies).
    AnalysisResult(const AnalysisResult&) = delete;
    auto operator=(const AnalysisResult&) -> AnalysisResult& = delete;

    // Scope tree root.
    [[nodiscard]] auto global_scope() -> Scope&;
    [[nodiscard]] auto global_scope() const -> const Scope&;

    // Flat indexes.
    std::unordered_map<std::string, ProcDef*> all_procs;
    std::unordered_map<std::string, VarDef*> all_variables;

    // Diagnostics.
    std::vector<Diagnostic> diagnostics;
    std::unordered_map<int32_t, std::unordered_set<std::string>> suppressed_lines;

    // Semantic facts.
    std::vector<RegexPattern> regex_patterns;
    std::vector<CommandInvocation> command_invocations;
    std::vector<PackageRequire> package_requires;
    std::vector<PackageProvide> package_provides;
    bool has_dynamic_providers = false;
    std::vector<SourceTarget> source_targets;
    std::vector<StubCommandDef> stub_commands;
    std::vector<StubExprDef> stub_expr_defs;

    // Command aliases: qualified alias name -> (target_cmd, prepended_args).
    std::unordered_map<std::string, std::pair<std::string, std::vector<std::string>>>
        command_aliases;

    std::optional<UnknownProcInfo> unknown_proc_info;

    // Lookup.
    [[nodiscard]] auto find_proc(std::string_view name) const -> const ProcDef*;
    [[nodiscard]] auto active_package_names() const -> std::unordered_set<std::string>;
    [[nodiscard]] auto package_context() const -> PackageContext;

    // Cached regex position set: {(line, character)} for all regex patterns.
    [[nodiscard]] auto regex_position_set() const -> const RegexPositionSet&;

    // Deep copy for incremental snapshots.
    [[nodiscard]] auto copy_for_snapshot() const -> AnalysisResult;

  private:
    std::unique_ptr<Scope> global_scope_;

    // Lazily built regex position cache.
    mutable std::optional<RegexPositionSet> regex_position_cache_;

    // Bare-name index for O(1) proc lookup fallback.
    mutable std::optional<std::unordered_map<std::string, const ProcDef*>> bare_name_index_;

    void ensure_bare_name_index() const;
    static auto copy_scope_tree(const Scope& src, Scope* parent) -> std::unique_ptr<Scope>;
};

} // namespace tcl_lsp
