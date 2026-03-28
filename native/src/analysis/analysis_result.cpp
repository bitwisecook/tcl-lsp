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
    for (const auto& [qname, proc] : all_procs) {
        bare_name_index_->try_emplace(proc->name, proc);
    }
}

auto AnalysisResult::find_proc(std::string_view name) const -> const ProcDef* {
    // Try qualified forms first.
    std::string with_prefix = "::" + std::string(name);
    if (auto it = all_procs.find(with_prefix); it != all_procs.end()) {
        return it->second;
    }
    if (auto it = all_procs.find(std::string(name)); it != all_procs.end()) {
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
    for (const auto& pr : package_requires) {
        result.insert(pr.name);
    }
    return result;
}

auto AnalysisResult::package_context() const -> PackageContext {
    PackageContext ctx;
    for (const auto& pr : package_requires) {
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
    for (const auto& pp : package_provides) {
        ctx.provided.insert(pp.name);
    }
    ctx.unknown_providers = has_dynamic_providers;
    return ctx;
}

auto AnalysisResult::regex_position_set() const -> const RegexPositionSet& {
    if (!regex_position_cache_.has_value()) {
        regex_position_cache_.emplace();
        for (const auto& rp : regex_patterns) {
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
    dest->has_body_range = src.has_body_range;

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
    // Walk the copied scope tree to find all procs and variables.
    struct Walker {
        std::unordered_map<std::string, ProcDef*>& all_procs;
        std::unordered_map<std::string, VarDef*>& all_variables;

        void walk(Scope& scope) {
            for (auto& [name, proc] : scope.procs) {
                all_procs[proc.qualified_name] = &proc;
            }
            for (auto& [name, var] : scope.variables) {
                all_variables[name] = &var;
            }
            for (const auto& child : scope.children) {
                walk(*child);
            }
        }
    };

    Walker walker{copy.all_procs, copy.all_variables};
    walker.walk(*copy.global_scope_);

    // Copy plain data.
    copy.diagnostics = diagnostics;
    copy.suppressed_lines = suppressed_lines;
    copy.regex_patterns = regex_patterns;
    copy.command_invocations = command_invocations;
    copy.package_requires = package_requires;
    copy.package_provides = package_provides;
    copy.has_dynamic_providers = has_dynamic_providers;
    copy.source_targets = source_targets;
    copy.stub_commands = stub_commands;
    copy.stub_expr_defs = stub_expr_defs;
    copy.command_aliases = command_aliases;
    copy.unknown_proc_info = unknown_proc_info;

    return copy;
}

} // namespace tcl_lsp
