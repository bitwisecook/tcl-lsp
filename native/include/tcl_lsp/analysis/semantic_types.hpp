#pragma once

#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

// How a proc parameter is used inside the proc body.
// Stored as a bitmask for compact set operations.
enum class ProcArgTrait : std::uint8_t {
    EVAL = 1 << 0,      // evaluated as a script (eval, uplevel, subst)
    BODY = 1 << 1,      // used as a loop/control body
    VAR_WRITE = 1 << 2, // names a variable that the proc writes (upvar + set)
    VAR_READ = 1 << 3,  // names a variable that the proc reads (upvar read-only)
    EXPR = 1 << 4,      // evaluated as an expression
    LOOP_LIST = 1 << 5, // used as the list in a foreach/lmap
};

// Bitfield of ProcArgTrait values.
using ProcArgTraits = std::uint8_t;

inline auto operator|(ProcArgTrait a, ProcArgTrait b) -> ProcArgTraits {
    return static_cast<ProcArgTraits>(a) | static_cast<ProcArgTraits>(b);
}

inline auto operator|(ProcArgTraits a, ProcArgTrait b) -> ProcArgTraits {
    return a | static_cast<ProcArgTraits>(b);
}

inline auto operator|=(ProcArgTraits& a, ProcArgTrait b) -> ProcArgTraits& {
    a = a | static_cast<ProcArgTraits>(b);
    return a;
}

inline auto has_trait(ProcArgTraits traits, ProcArgTrait t) -> bool {
    return (traits & static_cast<ProcArgTraits>(t)) != 0;
}

auto to_string(ProcArgTrait t) -> std::string;

// A parameter in a proc definition (immutable value).
struct ParamDef {
    std::string name;
    std::optional<std::string> default_value;

    auto operator==(const ParamDef&) const -> bool = default;
};

// A variable known to the analyser (mutable during analysis).
struct VarDef {
    std::string name;
    Range definition_range = Range::zero();
    std::vector<Range> references;
    bool warn_if_unused = false;
};

// A procedure defined via 'proc'.
struct ProcDef {
    std::string name;
    std::string qualified_name; // e.g. "::math::add"
    std::vector<ParamDef> params;
    Range name_range = Range::zero();
    Range body_range = Range::zero();
    std::string doc; // extracted from preceding comment
    std::unordered_map<std::string, ProcArgTraits> param_traits;
};

// Lexical scope kind.
enum class ScopeKind : std::uint8_t { GLOBAL, NAMESPACE, PROC };

auto to_string(ScopeKind k) -> std::string;

// A lexical scope (global, namespace, or proc body).
// Mutable during analysis.  Children are owned via unique_ptr; the tree
// is cleaned up automatically when the root scope is destroyed.
struct Scope {
    ScopeKind kind = ScopeKind::GLOBAL;
    std::string name;
    Scope* parent = nullptr;
    std::optional<Range> body_range;
    std::unordered_map<std::string, VarDef> variables;
    std::unordered_map<std::string, ProcDef> procs;
    std::vector<std::unique_ptr<Scope>> children;

    // Const-string tracking for regex variable propagation (Phase 4 analyser state).
    std::unordered_map<std::string, std::pair<std::string, Range>> const_strings;
    std::unordered_set<std::string> regex_vars;

    // Cached namespace path for this scope (computed lazily by Analyser).
    mutable std::string cached_namespace;

    Scope() = default;
    ~Scope() = default;

    // Movable (unique_ptr vector handles ownership transfer).
    Scope(Scope&&) noexcept = default;
    auto operator=(Scope&&) noexcept -> Scope& = default;

    // Not copyable (parent pointers would be wrong).
    Scope(const Scope&) = delete;
    auto operator=(const Scope&) -> Scope& = delete;
};

// Pre-order scope tree visitor.  Works with both mutable and const Scope.
template <typename F>
void visit_scope_tree(Scope& root, F&& fn) {
    fn(root);
    for (auto& child : root.children) {
        visit_scope_tree(*child, fn);
    }
}

template <typename F>
void visit_scope_tree(const Scope& root, F&& fn) {
    fn(root);
    for (const auto& child : root.children) {
        visit_scope_tree(*child, fn);
    }
}

} // namespace tcl_lsp
