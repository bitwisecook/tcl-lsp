#pragma once

#include <string_view>
#include <unordered_set>

namespace tcl_lsp {

// Standard Tcl command names recognised as keywords for semantic token
// classification.  This is a static approximation of the Python
// REGISTRY.command_names() set, covering core Tcl, Tk, TclOO, and
// definition-context words.
//
// At runtime the full set comes from the CommandRegistryInterface;
// this compile-time set is used when no registry is available (tests,
// standalone classification).

// Core command names that get "keyword" highlighting.
[[nodiscard]] auto is_keyword(std::string_view name) -> bool;

// Prefix math/comparison operators that get "operator" highlighting.
[[nodiscard]] auto is_operator(std::string_view name) -> bool;

} // namespace tcl_lsp
