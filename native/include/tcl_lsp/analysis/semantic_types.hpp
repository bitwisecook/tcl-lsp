#pragma once

#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <memory>
#include <string>
#include <unordered_map>
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

inline auto has_trait(ProcArgTraits traits, ProcArgTrait t) -> bool {
    return (traits & static_cast<ProcArgTraits>(t)) != 0;
}

auto to_string(ProcArgTrait t) -> std::string;

// A parameter in a proc definition (immutable value).
struct ParamDef {
    std::string name;
    bool has_default = false;
    std::string default_value;

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
    Range body_range = Range::zero();
    bool has_body_range = false;
    std::unordered_map<std::string, VarDef> variables;
    std::unordered_map<std::string, ProcDef> procs;
    std::vector<std::unique_ptr<Scope>> children;

    Scope() = default;
    ~Scope() = default;

    // Movable (unique_ptr vector handles ownership transfer).
    Scope(Scope&&) noexcept = default;
    auto operator=(Scope&&) noexcept -> Scope& = default;

    // Not copyable (parent pointers would be wrong).
    Scope(const Scope&) = delete;
    auto operator=(const Scope&) -> Scope& = delete;
};

} // namespace tcl_lsp
