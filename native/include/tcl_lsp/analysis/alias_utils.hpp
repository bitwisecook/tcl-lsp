#pragma once

#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

namespace tcl_lsp {

// Alias store: qualified_name -> (target_cmd, prepended_args).
using CommandAliasMap =
    std::unordered_map<std::string, std::pair<std::string, std::vector<std::string>>>;

// Return names that are aliases for "expr" (no prepended args).
// Returns both qualified keys ("::=") and stripped short names ("=").
[[nodiscard]] auto expr_alias_names(const CommandAliasMap& aliases)
    -> std::unordered_set<std::string>;

// Find an alias entry matching a bare command word.
// Tries "::word" (global qualified form).
[[nodiscard]] auto lookup_alias_for_word(
    std::string_view word,
    const CommandAliasMap& aliases)
    -> const std::pair<std::string, std::vector<std::string>>*;

} // namespace tcl_lsp
