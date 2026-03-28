#include "tcl_lsp/core/diagnostic.hpp"

#include <cstdio>

namespace tcl_lsp {

auto to_string(DiagCode c) -> std::string {
    auto val = static_cast<std::uint16_t>(c);
    char prefix = (val >= 2000) ? 'W' : 'E';
    int num = (val >= 2000) ? val - 2000 : val - 1000;
    char buf[8];
    std::snprintf(buf, sizeof(buf), "%c%03d", prefix, num);
    return std::string(buf);
}

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
