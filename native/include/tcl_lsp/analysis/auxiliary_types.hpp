#pragma once

#include "tcl_lsp/core/range.hpp"

#include <cstdint>
#include <string>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

// A source range known to contain a regular expression pattern.
struct RegexPattern {
    Range range;
    std::string pattern; // literal regex text
    std::string command; // originating command: "regexp", "regsub", "switch"

    auto operator==(const RegexPattern&) const -> bool = default;
};

// A command word observed during analysis.
struct CommandInvocation {
    std::string name;
    Range range;
    std::string resolved_qualified_name; // empty if unresolved

    auto operator==(const CommandInvocation&) const -> bool = default;
};

// A 'package require' invocation observed during analysis.
struct PackageRequire {
    std::string name;
    std::string version; // empty if unspecified
    Range range;
    bool conditional = false; // true if inside if/catch/try guard

    auto operator==(const PackageRequire&) const -> bool = default;
};

// A 'package provide' invocation observed during analysis.
struct PackageProvide {
    std::string name;
    std::string version; // empty if unspecified
    Range range;

    auto operator==(const PackageProvide&) const -> bool = default;
};

// A 'source' command target observed during analysis.
struct SourceTarget {
    std::string raw_path; // literal path text (may contain substitutions)
    Range range;
    bool is_literal = true; // false if path contains $ or [ substitutions

    auto operator==(const SourceTarget&) const -> bool = default;
};

// A parameter in a stub command definition.
struct StubArgDef {
    std::string name;
    std::string role = "value"; // body, expr, var, var_read, name, pattern, channel, value
    bool optional = false;

    auto operator==(const StubArgDef&) const -> bool = default;
};

// A command stub defined via '# tcl-lsp: stub' structured comment.
struct StubCommandDef {
    std::string name;
    std::vector<StubArgDef> args;
    Range range = Range::zero();
    bool barrier = false;
    bool loop = false;
    bool pure = false;
    bool mutator = false;
    bool unsafe = false;
    bool scope_alias = false;

    auto operator==(const StubCommandDef&) const -> bool = default;
};

// An expression function or operator stub.
struct StubExprDef {
    std::string name;
    std::string kind; // "function" or "operator"
    int32_t arity = 1;
    bool pure = true;
    Range range = Range::zero();

    auto operator==(const StubExprDef&) const -> bool = default;
};

// Analysis result from a user-defined 'unknown' proc.
struct UnknownProcInfo {
    std::unordered_set<std::string> dispatch_targets;
    bool chains_original = false;
    bool empty_stub = false;
    bool case_insensitive = false;
    bool has_pattern_dispatch = false;
    bool has_exec = false;
    bool has_auto_load = false;

    auto operator==(const UnknownProcInfo&) const -> bool = default;
};

// Diagnostic confidence level based on package detection certainty.
enum class Confidence : std::uint8_t { DEFINITE, PROBABLE, UNKNOWN };

// Packages active in a file, with confidence levels.
struct PackageContext {
    std::unordered_set<std::string> definite;  // unconditional 'package require'
    std::unordered_set<std::string> probable;  // conditional requires (if/catch guards)
    std::unordered_set<std::string> provided;  // 'package provide' in this file
    bool unknown_providers = false;

    [[nodiscard]] auto all_required() const -> std::unordered_set<std::string>;
    [[nodiscard]] auto confidence(const std::string& package) const -> Confidence;
};

} // namespace tcl_lsp
