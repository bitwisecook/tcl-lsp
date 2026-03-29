#include "tcl_lsp/registry/native_registry_adapter.hpp"
#include "tcl_lsp/registry/command_registry.hpp"

// Include generated command descriptors and hover table.
#include "generated/all_commands.cpp"

namespace tcl_lsp {

// ─── Pimpl ─────────────────────────────────────────────────────────

struct NativeCommandRegistry::Impl {
    CommandRegistry registry;

    Impl() : registry(generated::all_commands, generated::hover_table) {}
};

// ─── Construction ──────────────────────────────────────────────────

NativeCommandRegistry::NativeCommandRegistry()
    : impl_(std::make_unique<Impl>()) {}

NativeCommandRegistry::~NativeCommandRegistry() = default;

// ─── CommandRegistryInterface ──────────────────────────────────────

auto NativeCommandRegistry::command_names() const -> std::vector<std::string> {
    std::vector<std::string> names;
    names.reserve(impl_->registry.size());
    for (const auto* desc : impl_->registry.all_for_dialect(DialectFlags::ALL)) {
        names.emplace_back(desc->name);
    }
    return names;
}

auto NativeCommandRegistry::validation(std::string_view cmd) const
    -> std::optional<Arity> {
    const auto* desc = impl_->registry.find(cmd);
    if (desc == nullptr) return std::nullopt;
    return desc->arity;
}

auto NativeCommandRegistry::signature(std::string_view cmd) const
    -> std::optional<SignatureVariant> {
    const auto* desc = impl_->registry.find(cmd);
    if (desc == nullptr) return std::nullopt;

    // Subcommand-based command.
    if (!desc->subcommands.empty()) {
        SubcommandSig sub_sig;
        sub_sig.allow_unknown = desc->allow_unknown_subcommands;
        for (const auto& sub : desc->subcommands) {
            CommandSig sig;
            sig.arity = sub.arity;
            for (const auto& pat : sub.arg_patterns) {
                if (pat.kind == ArgPatternKind::FIXED && pat.index >= 0) {
                    sig.arg_roles[pat.index] = pat.role;
                }
            }
            sub_sig.subcommands[std::string(sub.name)] = std::move(sig);
        }
        return sub_sig;
    }

    // Simple command.
    CommandSig sig;
    sig.arity = desc->arity;
    for (const auto& pat : desc->arg_patterns) {
        if (pat.kind == ArgPatternKind::FIXED && pat.index >= 0) {
            sig.arg_roles[pat.index] = pat.role;
        }
    }
    return sig;
}

auto NativeCommandRegistry::is_known(std::string_view cmd) const -> bool {
    return impl_->registry.find(cmd) != nullptr;
}

auto NativeCommandRegistry::size() const -> std::size_t {
    return impl_->registry.size();
}

// ─── Singleton ─────────────────────────────────────────────────────

auto native_registry() -> const NativeCommandRegistry& {
    static const NativeCommandRegistry instance;
    return instance;
}

} // namespace tcl_lsp
