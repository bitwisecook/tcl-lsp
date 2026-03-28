#include "tcl_lsp/core/diagnostic.hpp"

namespace tcl_lsp {

auto to_string(Severity s) -> std::string {
    switch (s) {
    case Severity::ERROR: return "Error";
    case Severity::WARNING: return "Warning";
    case Severity::INFORMATION: return "Information";
    case Severity::HINT: return "Hint";
    }
    return "Unknown";
}

} // namespace tcl_lsp
