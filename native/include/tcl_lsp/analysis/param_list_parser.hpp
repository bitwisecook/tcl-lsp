#pragma once

#include "tcl_lsp/analysis/semantic_types.hpp"

#include <string_view>
#include <vector>

namespace tcl_lsp {

// Parse a Tcl proc argument list string into ParamDef objects.
// Handles: "a b c" and "a {b default} c" and "{a {}} {b 42} args".
[[nodiscard]] auto parse_param_list(std::string_view param_str) -> std::vector<ParamDef>;

} // namespace tcl_lsp
