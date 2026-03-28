#include "tcl_lsp/analysis/analysis_result.hpp"

#include <algorithm>

namespace tcl_lsp {

AnalysisResult::AnalysisResult()
    : global_scope_(std::make_unique<Scope>()) {
    global_scope_->kind = ScopeKind::GLOBAL;
    global_scope_->name = "::";
}

AnalysisResult::~AnalysisResult() = default;
AnalysisResult::AnalysisResult(AnalysisResult&&) noexcept = default;
auto AnalysisResult::operator=(AnalysisResult&&) noexcept -> AnalysisResult& = default;

auto AnalysisResult::global_scope() -> Scope& { return *global_scope_; }
auto AnalysisResult::global_scope() const -> const Scope& { return *global_scope_; }

void AnalysisResult::ensure_bare_name_index() const {
    if (bare_name_index_.has_value()) {
        return;
    }
    bare_name_index_.emplace();
    for (const auto& [qname, proc] : all_procs_) {
        bare_name_index_->try_emplace(proc->name, proc);
    }
}

auto AnalysisResult::find_proc(std::string_view name) const -> const ProcDef* {
    // Try qualified forms first.
    std::string with_prefix = "::" + std::string(name);
    if (auto it = all_procs_.find(with_prefix); it != all_procs_.end()) {
        return it->second;
    }
    if (auto it = all_procs_.find(std::string(name)); it != all_procs_.end()) {
        return it->second;
    }
    // Fall back to bare-name index.
    ensure_bare_name_index();
    if (auto it = bare_name_index_->find(std::string(name)); it != bare_name_index_->end()) {
        return it->second;
    }
    return nullptr;
}

auto AnalysisResult::active_package_names() const -> std::unordered_set<std::string> {
    std::unordered_set<std::string> result;
    for (const auto& pr : package_requires_) {
        result.insert(pr.name);
    }
    return result;
}

auto AnalysisResult::package_context() const -> PackageContext {
    PackageContext ctx;
    for (const auto& pr : package_requires_) {
        if (pr.conditional) {
            ctx.probable.insert(pr.name);
        } else {
            ctx.definite.insert(pr.name);
        }
    }
    // Unconditional overrides conditional for same package.
    for (const auto& name : ctx.definite) {
        ctx.probable.erase(name);
    }
    for (const auto& pp : package_provides_) {
        ctx.provided.insert(pp.name);
    }
    ctx.unknown_providers = has_dynamic_providers_;
    return ctx;
}

auto AnalysisResult::regex_position_set() const -> const RegexPositionSet& {
    if (!regex_position_cache_.has_value()) {
        regex_position_cache_.emplace();
        for (const auto& rp : regex_patterns_) {
            regex_position_cache_->emplace(rp.range.start.line, rp.range.start.character);
        }
    }
    return *regex_position_cache_;
}

auto AnalysisResult::copy_scope_tree(const Scope& src, Scope* parent)
    -> std::unique_ptr<Scope> {
    auto dest = std::make_unique<Scope>();
    dest->kind = src.kind;
    dest->name = src.name;
    dest->parent = parent;
    dest->body_range = src.body_range;

    // Copy variables (deep: VarDef references list).
    for (const auto& [name, var] : src.variables) {
        dest->variables.emplace(name, VarDef{var.name, var.definition_range,
                                             std::vector<Range>(var.references),
                                             var.warn_if_unused});
    }

    // Copy procs (deep: ParamDef list, param_traits map).
    for (const auto& [name, proc] : src.procs) {
        dest->procs.emplace(name, ProcDef{proc.name, proc.qualified_name,
                                          std::vector<ParamDef>(proc.params),
                                          proc.name_range, proc.body_range, proc.doc,
                                          std::unordered_map<std::string, ProcArgTraits>(
                                              proc.param_traits)});
    }

    // Recursively copy children.
    for (const auto& child_ptr : src.children) {
        dest->children.push_back(copy_scope_tree(*child_ptr, dest.get()));
    }

    return dest;
}

auto AnalysisResult::copy_for_snapshot() const -> AnalysisResult {
    AnalysisResult copy;
    copy.global_scope_ = copy_scope_tree(*global_scope_, nullptr);

    // Rebuild flat indexes pointing into the copied tree.
    visit_scope_tree(*copy.global_scope_, [&](Scope& scope) {
        for (auto& [name, proc] : scope.procs) {
            copy.all_procs_[proc.qualified_name] = &proc;
        }
        for (auto& [name, var] : scope.variables) {
            copy.all_variables_[scope.name + "::" + name] = &var;
        }
    });

    // Copy plain data.
    copy.diagnostics_ = diagnostics_;
    copy.suppressed_lines_ = suppressed_lines_;
    copy.regex_patterns_ = regex_patterns_;
    copy.command_invocations_ = command_invocations_;
    copy.package_requires_ = package_requires_;
    copy.package_provides_ = package_provides_;
    copy.has_dynamic_providers_ = has_dynamic_providers_;
    copy.source_targets_ = source_targets_;
    copy.stub_commands_ = stub_commands_;
    copy.stub_expr_defs_ = stub_expr_defs_;
    copy.command_aliases_ = command_aliases_;
    copy.unknown_proc_info_ = unknown_proc_info_;

    return copy;
}

} // namespace tcl_lsp
