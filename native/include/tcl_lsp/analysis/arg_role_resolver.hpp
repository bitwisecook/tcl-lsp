#pragma once

#include "tcl_lsp/analysis/command_interface.hpp"

#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

// Return the index of the first non-option argument.
// Scans args (0-based, command name excluded) skipping -option flags.
// value_options lists options that consume a following value argument.
[[nodiscard]] auto skip_options(std::span<const std::string> args,
                                const std::unordered_set<std::string>& value_options = {})
    -> std::int32_t;

// Return argument indices (0-based, after command name) for a single role.
[[nodiscard]] auto arg_indices_for_role(const CommandRegistryInterface* registry,
                                        std::string_view command,
                                        std::span<const std::string> args,
                                        ArgRole role) -> std::unordered_set<std::int32_t>;

// Return argument indices for multiple roles in one pass.
[[nodiscard]] auto arg_indices_for_roles(
    const CommandRegistryInterface* registry,
    std::string_view command,
    std::span<const std::string> args,
    std::span<const ArgRole> roles) -> std::vector<std::unordered_set<std::int32_t>>;

// Special-purpose index helpers for semantic tokens.

// Which argument index (0-based after cmd name) holds the proc parameter list?
// Returns nullopt if not applicable.
[[nodiscard]] auto proc_param_list_arg_index(std::string_view cmd,
                                              std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument index holds the procedure name being defined?
[[nodiscard]] auto procedure_name_arg_index(std::string_view cmd,
                                             std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument index holds the subcommand word?
[[nodiscard]] auto subcommand_arg_index(std::string_view cmd,
                                         std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument index holds the binary format/scan specifier string?
[[nodiscard]] auto binary_format_arg_index(std::string_view cmd,
                                            std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument index holds the sprintf/format format string?
[[nodiscard]] auto sprintf_format_arg_index(std::string_view cmd,
                                             std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument index holds the clock format string?
[[nodiscard]] auto clock_format_arg_index(std::string_view cmd,
                                           std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument index holds the regsub substitution spec?
[[nodiscard]] auto regsub_subspec_arg_index(std::string_view cmd,
                                             std::span<const std::string> argv)
    -> std::optional<std::int32_t>;

// Which argument indices are option/flag positions?
[[nodiscard]] auto option_arg_indices(std::string_view cmd,
                                       std::span<const std::string> args)
    -> std::unordered_set<std::int32_t>;

// Which argument index holds the regexp pattern for regexp/regsub?
[[nodiscard]] auto regexp_pattern_index(std::span<const std::string> args) -> std::optional<std::int32_t>;

// Is the given subcommand name known for this command?
[[nodiscard]] auto is_known_subcommand(const CommandRegistryInterface* registry,
                                        std::string_view cmd,
                                        std::string_view sub) -> bool;

} // namespace tcl_lsp
