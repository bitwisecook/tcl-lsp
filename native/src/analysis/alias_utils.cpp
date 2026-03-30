#include "tcl_lsp/analysis/alias_utils.hpp"

#include <string>
#include <string_view>
#include <vector>

namespace tcl_lsp {

namespace {

// Normalise a qualified Tcl name: split on ::, remove empties, rebuild with :: prefix.
auto normalise_qualified(const std::string& name) -> std::string {
    if (name.empty()) return name;
    std::vector<std::string_view> parts;
    std::string_view sv = name;
    while (!sv.empty()) {
        auto sep = sv.find("::");
        if (sep == std::string_view::npos) {
            if (!sv.empty()) parts.push_back(sv);
            break;
        }
        auto part = sv.substr(0, sep);
        if (!part.empty()) parts.push_back(part);
        sv = sv.substr(sep + 2);
    }
    if (parts.empty()) return "::";
    std::string result = "::";
    for (std::size_t i = 0; i < parts.size(); ++i) {
        if (i > 0) result += "::";
        result += parts[i];
    }
    return result;
}

} // namespace

auto expr_alias_names(const CommandAliasMap& aliases)
    -> std::unordered_set<std::string> {
    std::unordered_set<std::string> result;
    for (const auto& [name, entry] : aliases) {
        const auto& [target, prepended] = entry;
        if (target == "expr" && prepended.empty()) {
            result.insert(name);
            if (name.starts_with("::")) {
                auto pos = name.rfind("::");
                if (pos != std::string::npos && pos + 2 < name.size()) {
                    result.insert(name.substr(pos + 2));
                }
            }
        }
    }
    return result;
}

auto lookup_alias_for_word(
    std::string_view word,
    const CommandAliasMap& aliases)
    -> const std::pair<std::string, std::vector<std::string>>* {
    auto qualified = normalise_qualified("::" + std::string(word));
    auto it = aliases.find(qualified);
    if (it != aliases.end()) return &it->second;
    return nullptr;
}

} // namespace tcl_lsp
