#include "tcl_lsp/analysis/auxiliary_types.hpp"

namespace tcl_lsp {

auto PackageContext::all_required() const -> std::unordered_set<std::string> {
    auto result = definite;
    result.insert(probable.begin(), probable.end());
    return result;
}

auto PackageContext::confidence(const std::string& package) const -> Confidence {
    if (definite.contains(package)) {
        return Confidence::DEFINITE;
    }
    if (probable.contains(package)) {
        return Confidence::PROBABLE;
    }
    return Confidence::UNKNOWN;
}

} // namespace tcl_lsp
