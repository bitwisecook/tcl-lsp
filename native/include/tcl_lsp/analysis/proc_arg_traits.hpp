#pragma once

#include "tcl_lsp/analysis/command_interface.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"

#include <span>
#include <string>
#include <unordered_map>

namespace tcl_lsp {

// Shallow single-pass trait inference: scans top-level commands in body_source
// for patterns like eval $param, upvar 1 $param local, foreach x $list body.
// Returns traits only for params that have detected traits (empty map if none).
[[nodiscard]] auto infer_param_traits_shallow(
    std::span<const std::string> params,
    std::string_view body_source,
    const CommandRegistryInterface* registry = nullptr)
    -> std::unordered_map<std::string, ProcArgTraits>;

} // namespace tcl_lsp
