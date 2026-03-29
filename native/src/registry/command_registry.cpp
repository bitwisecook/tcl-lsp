#include "tcl_lsp/registry/command_registry.hpp"

#include <algorithm>
#include <string>

namespace tcl_lsp {

// ─── Construction ──────────────────────────────────────────────────

CommandRegistry::CommandRegistry(std::span<const CommandDesc* const> descriptors) {
    for (const auto* desc : descriptors) {
        if (desc != nullptr) {
            commands_.emplace(desc->name, desc);
        }
    }
}

CommandRegistry::CommandRegistry(std::span<const CommandDesc* const> descriptors,
                                 std::span<const HoverText> hover_table)
    : hover_table_(hover_table) {
    for (const auto* desc : descriptors) {
        if (desc != nullptr) {
            commands_.emplace(desc->name, desc);
        }
    }
}

// ─── Lookup ────────────────────────────────────────────────────────

auto CommandRegistry::find(std::string_view name) const -> const CommandDesc* {
    auto it = commands_.find(name);
    if (it != commands_.end()) {
        return it->second;
    }
    return nullptr;
}

auto CommandRegistry::find(std::string_view name, DialectFlags dialect) const
    -> const CommandDesc* {
    const auto* desc = find(name);
    if (desc == nullptr) return nullptr;

    // ALL-dialect commands are visible everywhere.
    if (desc->dialects == DialectFlags::ALL) return desc;

    // Check if the command's dialect set overlaps with the visibility set.
    auto vis = dialect_visibility(dialect);
    if ((desc->dialects & vis) != DialectFlags::NONE) return desc;

    return nullptr;
}

auto CommandRegistry::all_for_dialect(DialectFlags dialect) const
    -> std::vector<const CommandDesc*> {
    auto vis = dialect_visibility(dialect);
    std::vector<const CommandDesc*> result;
    result.reserve(commands_.size());
    for (const auto& [_, desc] : commands_) {
        if (desc->dialects == DialectFlags::ALL ||
            (desc->dialects & vis) != DialectFlags::NONE) {
            result.push_back(desc);
        }
    }
    return result;
}

// ─── Hover lookup ─────────────────────────────────────────────────

auto CommandRegistry::hover_at(std::int32_t index) const -> const HoverText* {
    if (index < 0 || static_cast<std::size_t>(index) >= hover_table_.size()) {
        return nullptr;
    }
    return &hover_table_[static_cast<std::size_t>(index)];
}

auto CommandRegistry::hover_for(const CommandDesc& cmd) const -> const HoverText* {
    return hover_at(cmd.hover_index);
}

auto CommandRegistry::hover_for(const SubCmdDesc& sub) const -> const HoverText* {
    return hover_at(sub.hover_index);
}

// ─── Static pattern resolution ─────────────────────────────────────
// Apply ArgPattern entries to fill roles.

namespace {

void apply_patterns(
    ResolveResult& result,
    std::span<const ArgPattern> patterns,
    std::size_t argc) {
    for (const auto& pat : patterns) {
        switch (pat.kind) {
        case ArgPatternKind::FIXED: {
            auto idx = pat.index;
            // -1 means last argument.
            if (idx < 0) idx = static_cast<std::int16_t>(argc) + idx;
            if (idx >= 0 && static_cast<std::size_t>(idx) < argc) {
                result.set(static_cast<std::size_t>(idx), pat.role);
            }
            break;
        }
        case ArgPatternKind::TAIL: {
            auto start = static_cast<std::size_t>(pat.index);
            for (std::size_t i = start; i < argc; ++i) {
                result.set(i, pat.role);
            }
            break;
        }
        case ArgPatternKind::STRIDE: {
            if (pat.stride <= 0) break;
            auto start = static_cast<std::size_t>(pat.index);
            auto step = static_cast<std::size_t>(pat.stride);
            for (std::size_t i = start; i < argc; i += step) {
                result.set(i, pat.role);
            }
            break;
        }
        case ArgPatternKind::OPTION_VALUE: {
            // Scan args for the option name, mark the following arg.
            for (std::size_t i = 0; i + 1 < argc; ++i) {
                // Note: args are not available in this context — OPTION_VALUE
                // patterns require the actual arg strings. This case is handled
                // in the full resolve_arg_roles below. Here we skip it.
            }
            break;
        }
        }
    }
}

void apply_option_value_patterns(
    ResolveResult& result,
    std::span<const ArgPattern> patterns,
    std::span<const std::string_view> args) {
    for (const auto& pat : patterns) {
        if (pat.kind != ArgPatternKind::OPTION_VALUE) continue;
        for (std::size_t i = 0; i + 1 < args.size(); ++i) {
            if (args[i] == pat.option_name) {
                result.set(i + 1, pat.role);
            }
        }
    }
}

void apply_option_desc_roles(
    ResolveResult& result,
    std::span<const OptionDesc> options,
    std::span<const std::string_view> args) {
    // Options with value_role != VALUE get their value arg marked.
    for (const auto& opt : options) {
        if (!opt.takes_value || opt.value_role == ArgRole::VALUE) continue;
        for (std::size_t i = 0; i + 1 < args.size(); ++i) {
            if (args[i] == opt.name) {
                result.set(i + 1, opt.value_role);
            }
        }
    }
}

} // namespace

// ─── Layout resolvers ──────────────────────────────────────────────
// Each returns expected<ResolveResult, ResolveError>.
// These implement the same logic as the Python callables in runtime.py.

namespace {

auto resolve_if(std::span<const std::string_view> args) -> std::expected<ResolveResult, ResolveError> {
    // if expr ?then? body ?elseif expr ?then? body ...? ?else? ?body?
    ResolveResult result;
    result.resize(args.size());
    if (args.empty()) return result;

    std::size_t i = 0;
    while (i < args.size()) {
        if (i == 0 || (args[i] == "elseif")) {
            if (args[i] == "elseif") {
                result.set(i, ArgRole::VALUE); // keyword
                ++i;
            }
            // expr
            if (i < args.size()) {
                result.set(i, ArgRole::EXPR);
                ++i;
            }
            // optional "then"
            if (i < args.size() && args[i] == "then") {
                result.set(i, ArgRole::VALUE);
                ++i;
            }
            // body
            if (i < args.size()) {
                result.set(i, ArgRole::BODY);
                ++i;
            }
        } else if (args[i] == "else") {
            result.set(i, ArgRole::VALUE); // keyword
            ++i;
            if (i < args.size()) {
                result.set(i, ArgRole::BODY);
                ++i;
            }
        } else {
            // Bare else body (no "else" keyword).
            result.set(i, ArgRole::BODY);
            ++i;
        }
    }
    return result;
}

auto resolve_switch(std::span<const std::string_view> args)
    -> std::expected<ResolveResult, ResolveError> {
    // switch ?opts? string pattern body ?pattern body ...?
    ResolveResult result;
    result.resize(args.size());

    // Skip options.
    std::size_t i = 0;
    while (i < args.size() && !args[i].empty() && args[i][0] == '-') {
        result.set(i, ArgRole::OPTION);
        if (args[i] == "--") {
            ++i;
            break;
        }
        ++i;
    }

    // String to match.
    if (i < args.size()) {
        result.set(i, ArgRole::VALUE);
        ++i;
    }

    // If only one arg remains, it's a body containing pattern-body pairs.
    if (i + 1 == args.size()) {
        result.set(i, ArgRole::BODY);
        return result;
    }

    // Alternating pattern body pairs.
    while (i + 1 < args.size()) {
        result.set(i, ArgRole::PATTERN);
        ++i;
        result.set(i, ArgRole::BODY);
        ++i;
    }
    // Lone trailing pattern (error case, but handle gracefully).
    if (i < args.size()) {
        result.set(i, ArgRole::PATTERN);
    }
    return result;
}

auto resolve_try(std::span<const std::string_view> args)
    -> std::expected<ResolveResult, ResolveError> {
    // try body ?on code varList body? ?trap pattern varList body? ?finally body?
    ResolveResult result;
    result.resize(args.size());
    if (args.empty()) return result;

    // First arg is always body.
    result.set(0, ArgRole::BODY);
    std::size_t i = 1;

    while (i < args.size()) {
        if (args[i] == "on" || args[i] == "trap") {
            result.set(i, ArgRole::VALUE); // keyword
            ++i;
            // code/pattern
            if (i < args.size()) {
                result.set(i, args[i - 1] == "trap" ? ArgRole::PATTERN : ArgRole::VALUE);
                ++i;
            }
            // varList
            if (i < args.size()) {
                result.set(i, ArgRole::VALUE);
                ++i;
            }
            // body
            if (i < args.size()) {
                result.set(i, ArgRole::BODY);
                ++i;
            }
        } else if (args[i] == "finally") {
            result.set(i, ArgRole::VALUE); // keyword
            ++i;
            if (i < args.size()) {
                result.set(i, ArgRole::BODY);
                ++i;
            }
        } else {
            ++i;
        }
    }
    return result;
}

auto resolve_when(std::span<const std::string_view> args)
    -> std::expected<ResolveResult, ResolveError> {
    // when EVENT ?priority N? ?timing enable|disable? body
    // Body is always the last arg.
    ResolveResult result;
    result.resize(args.size());
    if (args.empty()) return result;

    result.set(0, ArgRole::NAME); // event name
    if (args.size() > 1) {
        result.set(args.size() - 1, ArgRole::BODY);
    }
    return result;
}

} // namespace

// ─── Main resolve_arg_roles ────────────────────────────────────────

auto CommandRegistry::resolve_arg_roles(
    const CommandDesc& cmd,
    std::span<const std::string_view> args,
    std::span<const bool> expand_flags) -> std::expected<ResolveResult, ResolveError> {

    // Check for {*} expansion.
    bool has_expansion = false;
    if (!expand_flags.empty()) {
        for (auto flag : expand_flags) {
            if (flag) {
                has_expansion = true;
                break;
            }
        }
    }

    // Dispatch to layout resolver if one is implemented.
    {
        std::optional<std::expected<ResolveResult, ResolveError>> resolved;
        switch (cmd.layout_resolver) {
        case LayoutResolver::IF:
            resolved = resolve_if(args);
            break;
        case LayoutResolver::SWITCH:
            resolved = resolve_switch(args);
            break;
        case LayoutResolver::TRY:
            resolved = resolve_try(args);
            break;
        case LayoutResolver::WHEN:
            resolved = resolve_when(args);
            break;
        case LayoutResolver::NONE:
        case LayoutResolver::FOREACH:
        case LayoutResolver::EXPECT:
        case LayoutResolver::TCLTEST:
        case LayoutResolver::OO_CLASS:
        case LayoutResolver::OO_DEFINE:
        case LayoutResolver::OO_DEFINITION:
        case LayoutResolver::OO_PRIVATE:
        case LayoutResolver::OO_SELF:
            // Not yet implemented — fall through to static pattern resolution.
            break;
        }
        if (resolved) {
            if (*resolved) {
                (*resolved)->has_expansion = has_expansion;
            }
            return *resolved;
        }
    }

    // Static pattern resolution.
    ResolveResult result;
    result.resize(args.size());
    result.has_expansion = has_expansion;

    apply_patterns(result, cmd.arg_patterns, args.size());
    apply_option_value_patterns(result, cmd.arg_patterns, args);
    apply_option_desc_roles(result, cmd.options, args);

    return result;
}

// ─── Hover rendering ───────────────────────────────────────────────

auto render_hover_markdown(const HoverText& h, std::string_view symbol) -> std::string {
    std::string parts;

    if (!h.synopsis.empty()) {
        parts += "```tcl\n";
        for (auto syn : h.synopsis) {
            parts += syn;
            parts += '\n';
        }
        parts += "```\n\n";
    } else if (!symbol.empty()) {
        parts += "```tcl\n";
        parts += symbol;
        parts += "\n```\n\n";
    }

    if (!h.summary.empty()) {
        parts += h.summary;
        parts += "\n\n";
    }

    if (!h.snippet.empty()) {
        parts += h.snippet;
        parts += "\n\n";
    }

    if (!h.return_value.empty()) {
        parts += "**Returns**: ";
        parts += h.return_value;
        parts += "\n\n";
    }

    if (!h.examples.empty()) {
        parts += "**Example**:\n```tcl\n";
        parts += h.examples;
        parts += "\n```\n\n";
    }

    if (!h.source.empty()) {
        parts += "_Source: ";
        parts += h.source;
        parts += "_\n";
    }

    // Trim trailing whitespace.
    while (!parts.empty() && (parts.back() == '\n' || parts.back() == ' ')) {
        parts.pop_back();
    }

    return parts;
}

auto render_hover_lean(const HoverText& h, std::string_view symbol) -> std::string {
    std::string parts;

    if (!h.synopsis.empty()) {
        parts += "```tcl\n";
        parts += h.synopsis[0];
        parts += "\n```";
    } else if (!symbol.empty()) {
        parts += "```tcl\n";
        parts += symbol;
        parts += "\n```";
    }

    if (!h.summary.empty()) {
        if (!parts.empty()) parts += "\n\n";
        parts += h.summary;
    }

    return parts;
}

} // namespace tcl_lsp
