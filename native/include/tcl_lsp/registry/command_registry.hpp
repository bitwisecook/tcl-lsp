#pragma once

#include "tcl_lsp/registry/command_desc.hpp"

#include <algorithm>
#include <cstdint>
#include <expected>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace tcl_lsp {

// ─── Resolve result ────────────────────────────────────────────────
// SmallVector-like inline storage for up to 16 arg roles without heap allocation.
// For most commands 16 is more than enough; dynamically grows for the rare case.
struct ResolveResult {
    static constexpr std::size_t inline_capacity = 16;

    ArgRole inline_storage[inline_capacity]{};
    std::vector<ArgRole> overflow{};
    std::size_t count = 0;
    bool has_expansion = false;

    [[nodiscard]] auto roles() const -> std::span<const ArgRole> {
        if (count <= inline_capacity) {
            return {inline_storage, count};
        }
        return overflow;
    }

    void resize(std::size_t n) {
        count = n;
        if (n > inline_capacity) {
            overflow.resize(n, ArgRole::VALUE);
        } else {
            for (std::size_t i = 0; i < n; ++i) {
                inline_storage[i] = ArgRole::VALUE;
            }
        }
    }

    void set(std::size_t i, ArgRole role) {
        if (count <= inline_capacity) {
            if (i < inline_capacity) inline_storage[i] = role;
        } else {
            if (i < overflow.size()) overflow[i] = role;
        }
    }

    [[nodiscard]] auto operator[](std::size_t i) const -> ArgRole {
        if (count <= inline_capacity) {
            return inline_storage[i];
        }
        return overflow[i];
    }
};

enum class ResolveError : std::uint8_t {
    UNKNOWN_LAYOUT,
    ARITY_MISMATCH,
};

// ─── Dialect visibility ────────────────────────────────────────────
// Expand a single dialect flag to all dialects it can see.
// iRules inherits from tcl8.4; TMSH from tcl8.4-8.6; Expect from tcl8.4-8.6.
[[nodiscard]] constexpr auto dialect_visibility(DialectFlags active) -> DialectFlags {
    using enum DialectFlags;
    // If multiple flags or ALL, return as-is.
    auto bits = static_cast<std::uint16_t>(active);
    if (bits == 0 || (bits & (bits - 1)) != 0) return active;

    // Single dialect flag — expand to include parent dialects.
    if ((active & IRULES) != NONE) return TCL84 | IRULES;
    if ((active & TMSH) != NONE) return TCL84 | TCL85 | TCL86 | TMSH;
    if ((active & EXPECT) != NONE) return TCL84 | TCL85 | TCL86 | EXPECT;
    if ((active & IAPPS) != NONE) return TCL84 | TCL85 | IAPPS;
    if ((active & TK) != NONE) return TCL_ALL | TK;
    // EDA dialects inherit from their base Tcl version.
    if ((active & EDA_SYNOPSYS) != NONE) return TCL84 | TCL85 | EDA_SYNOPSYS;
    if ((active & EDA_CADENCE) != NONE) return TCL84 | TCL85 | EDA_CADENCE;
    if ((active & EDA_XILINX) != NONE) return TCL84 | TCL85 | EDA_XILINX;
    if ((active & EDA_QUARTUS) != NONE) return TCL84 | TCL85 | EDA_QUARTUS;
    if ((active & EDA_MENTOR) != NONE) return TCL84 | TCL85 | EDA_MENTOR;
    return active;
}

// ─── Command registry ──────────────────────────────────────────────
class CommandRegistry {
  public:
    // Construct from a span of command descriptor pointers.
    explicit CommandRegistry(std::span<const CommandDesc* const> descriptors);

    // Look up a command by name, filtered by dialect visibility.
    [[nodiscard]] auto find(std::string_view name, DialectFlags dialect) const
        -> const CommandDesc*;

    // Look up a command by name without dialect filtering.
    [[nodiscard]] auto find(std::string_view name) const -> const CommandDesc*;

    // Return all commands visible in the given dialect.
    [[nodiscard]] auto all_for_dialect(DialectFlags dialect) const
        -> std::vector<const CommandDesc*>;

    // Total number of registered commands.
    [[nodiscard]] auto size() const -> std::size_t { return commands_.size(); }

    // Resolve argument roles for a command invocation.
    [[nodiscard]] static auto resolve_arg_roles(
        const CommandDesc& cmd,
        std::span<const std::string_view> args,
        std::span<const bool> expand_flags = {}) -> std::expected<ResolveResult, ResolveError>;

  private:
    // Sorted by name for binary search. Drop-in upgradeable to std::flat_map.
    struct Entry {
        std::string_view name;
        const CommandDesc* desc;

        [[nodiscard]] auto operator<(const Entry& other) const -> bool {
            return name < other.name;
        }
    };
    std::vector<Entry> commands_;
};

// ─── Hover rendering ───────────────────────────────────────────────
[[nodiscard]] auto render_hover_markdown(const HoverText& h, std::string_view symbol)
    -> std::string;
[[nodiscard]] auto render_hover_lean(const HoverText& h, std::string_view symbol)
    -> std::string;

} // namespace tcl_lsp
