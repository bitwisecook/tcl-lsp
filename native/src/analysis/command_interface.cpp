#include "tcl_lsp/analysis/command_interface.hpp"

namespace tcl_lsp {

void TestCommandRegistry::add_command(std::string name, Arity arity) {
    sigs_.insert_or_assign(std::move(name), CommandSig{arity, {}});
}

void TestCommandRegistry::add_command(std::string name, CommandSig sig) {
    sigs_.insert_or_assign(std::move(name), std::move(sig));
}

void TestCommandRegistry::add_subcommand(std::string name, SubcommandSig sig) {
    sigs_.insert_or_assign(std::move(name), std::move(sig));
}

auto TestCommandRegistry::command_names() const -> std::vector<std::string> {
    std::vector<std::string> names;
    names.reserve(sigs_.size());
    for (const auto& [name, _] : sigs_) {
        names.push_back(name);
    }
    return names;
}

auto TestCommandRegistry::validation(std::string_view cmd) const -> std::optional<Arity> {
    auto it = sigs_.find(std::string(cmd));
    if (it == sigs_.end()) {
        return std::nullopt;
    }
    if (const auto* cs = std::get_if<CommandSig>(&it->second)) {
        return cs->arity;
    }
    // SubcommandSig doesn't have a top-level arity.
    return std::nullopt;
}

auto TestCommandRegistry::signature(std::string_view cmd) const
    -> std::optional<SignatureVariant> {
    auto it = sigs_.find(std::string(cmd));
    if (it == sigs_.end()) {
        return std::nullopt;
    }
    return it->second;
}

auto TestCommandRegistry::is_known(std::string_view cmd) const -> bool {
    return sigs_.contains(std::string(cmd));
}

} // namespace tcl_lsp
