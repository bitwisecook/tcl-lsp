#include "tcl_lsp/analysis/semantic_types.hpp"

namespace tcl_lsp {

auto to_string(ProcArgTrait t) -> std::string {
    switch (t) {
    case ProcArgTrait::EVAL: return "EVAL";
    case ProcArgTrait::BODY: return "BODY";
    case ProcArgTrait::VAR_WRITE: return "VAR_WRITE";
    case ProcArgTrait::VAR_READ: return "VAR_READ";
    case ProcArgTrait::EXPR: return "EXPR";
    case ProcArgTrait::LOOP_LIST: return "LOOP_LIST";
    }
    return "Unknown";
}

auto to_string(ScopeKind k) -> std::string {
    switch (k) {
    case ScopeKind::GLOBAL: return "global";
    case ScopeKind::NAMESPACE: return "namespace";
    case ScopeKind::PROC: return "proc";
    }
    return "unknown";
}

} // namespace tcl_lsp
