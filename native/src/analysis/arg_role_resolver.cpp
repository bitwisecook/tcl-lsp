#include "tcl_lsp/analysis/arg_role_resolver.hpp"

#include <algorithm>
#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <unordered_set>
#include <variant>
#include <vector>

namespace tcl_lsp {
namespace {

// TclOO definition-context subcommands.
const std::unordered_set<std::string_view> OO_DEFINE_SUBCOMMANDS = {
    "constructor", "destructor", "method",    "forward",   "mixin",
    "filter",      "superclass", "export",    "unexport",  "self",
    "renamemethod", "deletemethod", "variable",
};

auto oo_class_object_body_indices(std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    if (args.size() < 2) return {};
    auto& sub = args[0];
    if (sub == "create" && args.size() >= 3) return {2};
    if (sub == "new" && args.size() >= 2) return {1};
    return {};
}

auto oo_define_body_indices(std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    // Script form: oo::define Target { body }
    if (args.size() == 2 && OO_DEFINE_SUBCOMMANDS.find(args[1]) == OO_DEFINE_SUBCOMMANDS.end())
        return {1};
    if (args.size() < 2) return {};

    auto& sub = args[1];
    if (sub == "constructor" && args.size() >= 4) return {3};
    if (sub == "destructor" && args.size() >= 3) return {2};
    if (sub == "method" && args.size() >= 5) return {4};
    if (sub == "self" && args.size() >= 3) {
        auto& self_sub = args[2];
        if (self_sub == "constructor" && args.size() >= 5) return {4};
        if (self_sub == "destructor" && args.size() >= 4) return {3};
        if (self_sub == "method" && args.size() >= 6) return {5};
    }
    return {};
}

auto oo_definition_body_indices(std::string_view command, std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    if (command == "constructor" && args.size() >= 2) return {1};
    if (command == "destructor" && !args.empty()) return {0};
    if (command == "method" && args.size() >= 3) return {2};
    if (command == "self" && !args.empty()) {
        auto& sub = args[0];
        if (sub == "constructor" && args.size() >= 3) return {2};
        if (sub == "destructor" && args.size() >= 2) return {1};
        if (sub == "method" && args.size() >= 4) return {3};
    }
    return {};
}

auto if_body_indices(std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    std::unordered_set<std::int32_t> result;
    auto sz = static_cast<std::int32_t>(args.size());
    std::int32_t i = 0;

    // Initial: if expr ?then? body
    if (i < sz) i += 1; // expr
    if (i < sz && args[static_cast<std::size_t>(i)] == "then") i += 1;
    if (i < sz) { result.insert(i); i += 1; }

    // Repeated: elseif expr ?then? body ... else body
    while (i < sz) {
        auto& kw = args[static_cast<std::size_t>(i)];
        if (kw == "elseif") {
            i += 1; // keyword
            if (i < sz) i += 1; // expr
            if (i < sz && args[static_cast<std::size_t>(i)] == "then") i += 1;
            if (i < sz) { result.insert(i); i += 1; }
            continue;
        }
        if (kw == "else") {
            if (i + 1 < sz) result.insert(i + 1);
            break;
        }
        i += 1;
    }
    return result;
}

auto if_expr_indices(std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    std::unordered_set<std::int32_t> result;
    auto sz = static_cast<std::int32_t>(args.size());
    std::int32_t i = 0;

    if (i < sz) { result.insert(i); i += 1; }
    if (i < sz && args[static_cast<std::size_t>(i)] == "then") i += 1;
    if (i < sz) i += 1; // body

    while (i < sz) {
        auto& kw = args[static_cast<std::size_t>(i)];
        if (kw == "elseif") {
            i += 1;
            if (i < sz) { result.insert(i); i += 1; }
            if (i < sz && args[static_cast<std::size_t>(i)] == "then") i += 1;
            if (i < sz) i += 1; // body
            continue;
        }
        if (kw == "else") break;
        i += 1;
    }
    return result;
}

auto try_body_indices(std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    std::unordered_set<std::int32_t> result;
    auto sz = static_cast<std::int32_t>(args.size());
    if (!args.empty()) result.insert(0);

    std::int32_t i = 1;
    while (i < sz) {
        auto& kw = args[static_cast<std::size_t>(i)];
        if (kw == "finally") {
            if (i + 1 < sz) result.insert(i + 1);
            i += 2;
        } else if (kw == "on" || kw == "trap") {
            if (i + 3 < sz) result.insert(i + 3);
            i += 4;
        } else {
            i += 1;
        }
    }
    return result;
}

auto switch_body_indices(std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    std::unordered_set<std::int32_t> result;
    auto sz = static_cast<std::int32_t>(args.size());
    std::int32_t i = 0;

    // Skip options.
    while (i < sz && !args[static_cast<std::size_t>(i)].empty() &&
           args[static_cast<std::size_t>(i)][0] == '-') {
        if (args[static_cast<std::size_t>(i)] == "--") { i += 1; break; }
        i += 1;
    }

    // Skip switch string.
    if (i >= sz) return result;
    i += 1;
    if (i >= sz) return result;

    // Braced list form: single trailing argument.
    if (i == sz - 1) { result.insert(i); return result; }

    // List form: pattern body pattern body ...
    while (i + 1 < sz) {
        if (args[static_cast<std::size_t>(i + 1)] != "-")
            result.insert(i + 1);
        i += 2;
    }
    return result;
}

auto resolve_from_sig(const SignatureVariant& sig,
                       std::span<const std::string> args,
                       ArgRole role,
                       std::string_view command)
    -> std::unordered_set<std::int32_t> {
    auto sz = static_cast<std::int32_t>(args.size());

    if (auto* cmd_sig = std::get_if<CommandSig>(&sig)) {
        std::unordered_set<std::int32_t> result;
        for (auto& [idx, arg_role] : cmd_sig->arg_roles) {
            if (arg_role == role && idx < sz) result.insert(idx);
        }
        // Special case: foreach last arg is always BODY.
        if (role == ArgRole::BODY && (command == "foreach" || command == "lmap") && args.size() >= 3)
            result.insert(sz - 1);
        return result;
    }

    if (auto* sub_sig = std::get_if<SubcommandSig>(&sig)) {
        if (args.empty()) return {};
        auto it = sub_sig->subcommands.find(std::string(args[0]));
        if (it == sub_sig->subcommands.end()) return {};
        std::unordered_set<std::int32_t> result;
        for (auto& [idx, arg_role] : it->second.arg_roles) {
            if (arg_role == role && (idx + 1) < sz) result.insert(idx + 1);
        }
        return result;
    }

    return {};
}

} // anonymous namespace

auto skip_options(std::span<const std::string> args,
                   const std::unordered_set<std::string>& value_options)
    -> std::int32_t {
    std::int32_t i = 0;
    auto sz = static_cast<std::int32_t>(args.size());
    while (i < sz) {
        auto& arg = args[static_cast<std::size_t>(i)];
        if (arg == "--") { i += 1; break; }
        if (!arg.empty() && arg[0] == '-') {
            i += 1;
            if (value_options.count(arg) && i < sz) i += 1;
            continue;
        }
        break;
    }
    return i;
}

auto arg_indices_for_role(const CommandRegistryInterface* registry,
                           std::string_view command,
                           std::span<const std::string> args,
                           ArgRole role) -> std::unordered_set<std::int32_t> {
    if (role == ArgRole::BODY) {
        if (command == "oo::class" || command == "oo::object")
            return oo_class_object_body_indices(args);
        if (command == "oo::define" || command == "oo::objdefine")
            return oo_define_body_indices(args);
        auto oo = oo_definition_body_indices(command, args);
        if (!oo.empty()) return oo;
        if (command == "when" && args.size() >= 2)
            return {static_cast<std::int32_t>(args.size()) - 1};
        if (command == "if") return if_body_indices(args);
        if (command == "try") return try_body_indices(args);
        if (command == "switch") return switch_body_indices(args);
        if ((command == "foreach" || command == "lmap") && args.size() >= 3)
            return {static_cast<std::int32_t>(args.size()) - 1};
        // Hardcoded fallbacks for common commands (no registry needed).
        if (command == "proc" && args.size() >= 3) return {2};
        if (command == "while" && args.size() >= 2) return {1};
        if (command == "for" && args.size() >= 4) return {0, 2, 3};
        if ((command == "catch" || command == "eval" || command == "uplevel") && !args.empty())
            return {0};
        if (command == "namespace" && !args.empty() && args[0] == "eval" && args.size() >= 3)
            return {2};
        if (command == "apply" && !args.empty()) return {0};
        if (command == "after" && args.size() >= 2 && args[0] != "cancel" && args[0] != "idle" && args[0] != "info")
            return {static_cast<std::int32_t>(args.size()) - 1};
    }

    if (role == ArgRole::EXPR) {
        if (command == "if") return if_expr_indices(args);
        // Hardcoded: expr arg 0 is an expression.
        if (command == "expr" && !args.empty()) return {0};
        if (command == "while" && !args.empty()) return {0};
        if (command == "for" && args.size() >= 2) return {1};
    }

    if (role == ArgRole::VAR_NAME) {
        // Hardcoded: set, incr, variable, global, upvar target vars.
        if ((command == "set" || command == "incr" || command == "append" ||
             command == "lappend") && !args.empty())
            return {0};
        if (command == "variable" || command == "global") {
            std::unordered_set<std::int32_t> result;
            for (std::size_t i = 0; i < args.size(); i += 2)
                result.insert(static_cast<std::int32_t>(i));
            return result;
        }
        if (command == "upvar") {
            // upvar ?level? otherVar myVar ?otherVar myVar ...?
            std::unordered_set<std::int32_t> result;
            std::size_t start = 0;
            if (!args.empty() && !args[0].empty() &&
                (args[0][0] == '#' || std::isdigit(static_cast<unsigned char>(args[0][0])) != 0))
                start = 1;
            for (std::size_t i = start + 1; i < args.size(); i += 2)
                result.insert(static_cast<std::int32_t>(i));
            return result;
        }
    }

    if (role == ArgRole::PATTERN) {
        if (command == "regexp" || command == "regsub") {
            auto idx = regexp_pattern_index(args);
            if (idx.has_value()) return {*idx};
            return {};
        }
    }

    if (registry == nullptr) return {};
    auto sig = registry->signature(command);
    if (!sig.has_value()) return {};
    return resolve_from_sig(*sig, args, role, command);
}

auto arg_indices_for_roles(
    const CommandRegistryInterface* registry,
    std::string_view command,
    std::span<const std::string> args,
    std::span<const ArgRole> roles) -> std::vector<std::unordered_set<std::int32_t>> {

    // Roles with special-case logic must go through per-role function.
    auto is_special = [](ArgRole r) {
        return r == ArgRole::BODY || r == ArgRole::EXPR || r == ArgRole::PATTERN ||
               r == ArgRole::VAR_NAME || r == ArgRole::VAR_READ;
    };

    std::vector<std::unordered_set<std::int32_t>> results(roles.size());
    std::vector<std::pair<std::size_t, ArgRole>> need_sig;

    for (std::size_t i = 0; i < roles.size(); i++) {
        if (is_special(roles[i])) {
            results[i] = arg_indices_for_role(registry, command, args, roles[i]);
        } else {
            need_sig.emplace_back(i, roles[i]);
        }
    }

    if (need_sig.empty()) return results;
    if (registry == nullptr) return results;

    auto sig = registry->signature(command);
    if (!sig.has_value()) return results;

    auto sz = static_cast<std::int32_t>(args.size());

    if (auto* sub_sig = std::get_if<SubcommandSig>(&*sig)) {
        if (args.empty()) return results;
        auto it = sub_sig->subcommands.find(std::string(args[0]));
        if (it == sub_sig->subcommands.end()) return results;
        for (auto& [result_idx, role] : need_sig) {
            for (auto& [idx, arg_role] : it->second.arg_roles) {
                if (arg_role == role && (idx + 1) < sz)
                    results[result_idx].insert(idx + 1);
            }
        }
    } else if (auto* cmd_sig = std::get_if<CommandSig>(&*sig)) {
        for (auto& [result_idx, role] : need_sig) {
            for (auto& [idx, arg_role] : cmd_sig->arg_roles) {
                if (arg_role == role && idx < sz)
                    results[result_idx].insert(idx);
            }
        }
    }

    return results;
}

auto proc_param_list_arg_index(std::string_view cmd, std::span<const std::string> argv)
    -> std::optional<std::int32_t> {
    // proc name params body → params is argv[2] (1-based: index 2 in full argv)
    // In the caller convention, argv includes cmd name, so params = index 2.
    if (cmd == "proc" && argv.size() >= 4) return 2;
    if ((cmd == "method" || cmd == "constructor") && argv.size() >= 3) return 1;
    if (cmd == "oo::define" || cmd == "oo::objdefine") {
        if (argv.size() >= 5 && argv[2] == "method") return 4;
        if (argv.size() >= 4 && argv[2] == "constructor") return 3;
    }
    return std::nullopt;
}

auto procedure_name_arg_index(std::string_view cmd, std::span<const std::string> argv)
    -> std::optional<std::int32_t> {
    if (cmd == "proc" && argv.size() >= 2) return 1;
    if (cmd == "method" && argv.size() >= 2) return 1;
    if (cmd == "oo::define" || cmd == "oo::objdefine") {
        if (argv.size() >= 4 && argv[2] == "method") return 3;
    }
    return std::nullopt;
}

auto subcommand_arg_index(std::string_view cmd, std::span<const std::string> argv)
    -> std::optional<std::int32_t> {
    // Commands with subcommands: dict, string, array, info, namespace, etc.
    // The subcommand is always argv[1] (index 1 in full argv).
    if (cmd == "dict" || cmd == "string" || cmd == "array" || cmd == "info" ||
        cmd == "namespace" || cmd == "interp" || cmd == "chan" || cmd == "file" ||
        cmd == "clock" || cmd == "binary" || cmd == "package" || cmd == "trace" ||
        cmd == "encoding" || cmd == "zlib" || cmd == "oo::class" ||
        cmd == "oo::object" || cmd == "oo::define" || cmd == "oo::objdefine") {
        if (argv.size() >= 2) return 1;
    }
    return std::nullopt;
}

auto binary_format_arg_index(std::string_view cmd, std::span<const std::string> argv)
    -> std::optional<std::int32_t> {
    if (cmd == "binary" && argv.size() >= 3) {
        if (argv[1] == "format" || argv[1] == "scan") return 2;
    }
    return std::nullopt;
}

auto sprintf_format_arg_index(std::string_view cmd, std::span<const std::string> argv)
    -> std::optional<std::int32_t> {
    if (cmd == "format" && argv.size() >= 2) return 1;
    if (cmd == "scan" && argv.size() >= 3) return 2;
    return std::nullopt;
}

auto clock_format_arg_index(std::string_view cmd, std::span<const std::string> argv)
    -> std::optional<std::int32_t> {
    if (cmd != "clock" || argv.size() < 3) return std::nullopt;
    if (argv[1] != "format" && argv[1] != "scan") return std::nullopt;
    // clock format value -format fmtString
    for (std::size_t i = 3; i < argv.size(); i++) {
        if (argv[i] == "-format" && i + 1 < argv.size())
            return static_cast<std::int32_t>(i + 1);
    }
    return std::nullopt;
}

auto regsub_subspec_arg_index(std::string_view cmd, std::span<const std::string> args)
    -> std::optional<std::int32_t> {
    if (cmd != "regsub") return std::nullopt;
    // regsub ?options? pattern string subSpec ?varName?
    // Skip options, then: pattern, string, subSpec.
    // -start takes a value argument.
    static const std::unordered_set<std::string> value_opts{"-start"};
    auto i = skip_options(args, value_opts);
    // pattern
    if (static_cast<std::size_t>(i) >= args.size()) return std::nullopt;
    i += 1;
    // string
    if (static_cast<std::size_t>(i) >= args.size()) return std::nullopt;
    i += 1;
    // subSpec
    if (static_cast<std::size_t>(i) >= args.size()) return std::nullopt;
    return i;
}

auto option_arg_indices(std::string_view /*cmd*/,
                         std::span<const std::string> args)
    -> std::unordered_set<std::int32_t> {
    std::unordered_set<std::int32_t> result;
    for (std::size_t i = 0; i < args.size(); i++) {
        if (!args[i].empty() && args[i][0] == '-' && args[i] != "--") {
            result.insert(static_cast<std::int32_t>(i));
        }
    }
    return result;
}

auto regexp_pattern_index(std::span<const std::string> args)
    -> std::optional<std::int32_t> {
    // -start takes a value argument; skip both the flag and its value.
    static const std::unordered_set<std::string> value_opts{"-start"};
    auto i = skip_options(args, value_opts);
    if (static_cast<std::size_t>(i) < args.size()) return i;
    return std::nullopt;
}

auto is_known_subcommand(const CommandRegistryInterface* registry,
                          std::string_view cmd, std::string_view sub) -> bool {
    if (registry == nullptr) return false;
    auto sig = registry->signature(cmd);
    if (!sig.has_value()) return false;
    auto* sub_sig = std::get_if<SubcommandSig>(&*sig);
    if (sub_sig == nullptr) return false;
    return sub_sig->subcommands.count(std::string(sub)) > 0;
}

} // namespace tcl_lsp
