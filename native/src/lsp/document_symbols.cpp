#include "tcl_lsp/lsp/document_symbols.hpp"

#include "tcl_lsp/analysis/semantic_types.hpp"

#include <string>
#include <vector>

namespace tcl_lsp {
namespace {

// Format proc parameter list for the detail string.
auto proc_detail(const ProcDef& proc) -> std::string {
    std::string detail = "(";
    for (std::size_t i = 0; i < proc.params.size(); i++) {
        if (i > 0) detail += ' ';
        auto& p = proc.params[i];
        if (p.default_value.has_value()) {
            detail += '{';
            detail += p.name;
            detail += ' ';
            detail += *p.default_value;
            detail += '}';
        } else {
            detail += p.name;
        }
    }
    detail += ')';
    return detail;
}

// Merge name_range and body_range to get the full extent of a proc.
auto merge_range(const Range& name_range, const Range& body_range) -> Range {
    Range result;
    result.start = name_range.start;
    // Use the later of body_range.end and name_range.end.
    if (body_range.end.line > name_range.end.line ||
        (body_range.end.line == name_range.end.line &&
         body_range.end.character > name_range.end.character)) {
        result.end = body_range.end;
    } else {
        result.end = name_range.end;
    }
    return result;
}

void collect_symbols(const Scope& scope, std::vector<DocumentSymbolInfo>& out) {
    // Procs in this scope.
    for (auto& [name, proc] : scope.procs) {
        DocumentSymbolInfo sym;
        sym.name = proc.name;
        sym.detail = proc_detail(proc);
        sym.kind = SymbolKind::FUNCTION;
        sym.range = merge_range(proc.name_range, proc.body_range);
        sym.selection_range = proc.name_range;
        out.push_back(std::move(sym));
    }

    // Namespace-level variables.
    if (scope.kind == ScopeKind::GLOBAL || scope.kind == ScopeKind::NAMESPACE) {
        for (auto& [name, var] : scope.variables) {
            DocumentSymbolInfo sym;
            sym.name = var.name;
            sym.kind = SymbolKind::VARIABLE;
            sym.range = var.definition_range;
            sym.selection_range = var.definition_range;
            out.push_back(std::move(sym));
        }
    }

    // Child scopes (namespaces become nested symbols).
    for (auto& child : scope.children) {
        if (child->kind == ScopeKind::NAMESPACE) {
            DocumentSymbolInfo ns_sym;
            ns_sym.name = child->name;
            ns_sym.kind = SymbolKind::NAMESPACE;
            if (child->body_range.has_value()) {
                ns_sym.range = *child->body_range;
                ns_sym.selection_range = *child->body_range;
            }
            collect_symbols(*child, ns_sym.children);
            out.push_back(std::move(ns_sym));
        } else {
            // Proc scopes: procs are already emitted above.
            // Recurse to find nested procs/namespaces inside proc bodies.
            collect_symbols(*child, out);
        }
    }
}

} // anonymous namespace

auto get_document_symbols(const AnalysisResult& analysis)
    -> std::vector<DocumentSymbolInfo> {
    std::vector<DocumentSymbolInfo> symbols;
    collect_symbols(analysis.global_scope(), symbols);
    return symbols;
}

} // namespace tcl_lsp
