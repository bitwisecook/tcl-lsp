#pragma once

// Static command registry — the canonical source of truth for
// every Tcl command's shape: what arguments mean (BODY, EXPR,
// VAR_NAME, PATTERN …), how many arguments are valid, and what
// subcommands exist.
//
// Each command lives in its own file under native/src/registry/tcl/.
// Open dict.cpp and you see exactly what "dict for" looks like.
// Every file is self-contained and validated against the Tcl man pages.

#include "tcl_lsp/analysis/command_interface.hpp"

#include <cstdint>
#include <initializer_list>
#include <limits>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace tcl_lsp {

// A single command's registration data: name + signature.
struct CommandEntry {
    std::string name;
    SignatureVariant sig;
};

// ─── Builder helpers ─────────────────────────────────────────────
//
// These exist solely for readability in per-command files.
// No runtime cost — they just construct the underlying types.

// Unlimited arity sentinel.
inline constexpr std::int32_t UNLIMITED = std::numeric_limits<std::int32_t>::max();

// Argument role map: index → role.
using ArgRoleMap = std::unordered_map<std::int32_t, ArgRole>;

// Build an Arity.  max defaults to UNLIMITED.
inline auto arity(std::int32_t min, std::int32_t max = UNLIMITED) -> Arity {
    return {min, max};
}

// Build an arg-role map from an initializer list.
inline auto roles(std::initializer_list<std::pair<const std::int32_t, ArgRole>> init)
    -> ArgRoleMap {
    return ArgRoleMap(init);
}

// Build a simple command entry (no subcommands).
inline auto command(std::string name, Arity ar, ArgRoleMap r = {}) -> CommandEntry {
    return {std::move(name), CommandSig{ar, std::move(r)}};
}

// Build one subcommand descriptor.
inline auto sub(std::string name, Arity ar, ArgRoleMap r = {})
    -> std::pair<std::string, CommandSig> {
    return {std::move(name), CommandSig{ar, std::move(r)}};
}

// Build a command-with-subcommands entry.
inline auto subcommands(std::string name,
                         std::initializer_list<std::pair<std::string, CommandSig>> subs)
    -> CommandEntry {
    SubcommandSig sig;
    for (auto& [sub_name, sub_sig] : subs)
        sig.subcommands.insert_or_assign(sub_name, sub_sig);
    return {std::move(name), std::move(sig)};
}

// Build a known-but-featureless command (just name + arity).
inline auto simple(std::string name, Arity ar = {0, UNLIMITED}) -> CommandEntry {
    return {std::move(name), CommandSig{ar, {}}};
}

// ─── StaticCommandRegistry ───────────────────────────────────────

class StaticCommandRegistry final : public CommandRegistryInterface {
  public:
    StaticCommandRegistry();

    [[nodiscard]] auto command_names() const -> std::vector<std::string> override;
    [[nodiscard]] auto validation(std::string_view cmd) const -> std::optional<Arity> override;
    [[nodiscard]] auto signature(std::string_view cmd) const
        -> std::optional<SignatureVariant> override;
    [[nodiscard]] auto is_known(std::string_view cmd) const -> bool override;

  private:
    std::unordered_map<std::string, SignatureVariant> sigs_;
};

// Singleton accessor — constructed once, never mutated.
[[nodiscard]] auto static_registry() -> const StaticCommandRegistry&;

} // namespace tcl_lsp
