#include "tcl_lsp/analysis/proc_arg_traits.hpp"

#include "tcl_lsp/analysis/arg_role_resolver.hpp"
#include "tcl_lsp/parsing/lexer.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"

#include <cctype>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

namespace {

// Extract a bare variable name from "$var" or "${var}".
auto extract_var_name(const std::string& text) -> std::optional<std::string> {
    if (text.size() < 2 || text[0] != '$') return std::nullopt;
    if (text[1] == '{') {
        auto close = text.find('}', 2);
        if (close == std::string::npos || close != text.size() - 1) return std::nullopt;
        auto name = text.substr(2, close - 2);
        if (name.empty()) return std::nullopt;
        return std::string(name);
    }
    // Simple $varName — must be all ident chars.
    for (std::size_t i = 1; i < text.size(); ++i) {
        char c = text[i];
        if (!std::isalnum(static_cast<unsigned char>(c)) && c != '_' && c != ':')
            return std::nullopt;
    }
    return text.substr(1);
}

// Variable-writing commands: command -> arg index of var name (0-based).
auto var_write_index(std::string_view cmd) -> std::optional<std::size_t> {
    if (cmd == "set" || cmd == "incr" || cmd == "append" ||
        cmd == "lappend" || cmd == "lset" || cmd == "unset")
        return 0;
    if (cmd == "gets") return 1;
    if (cmd == "global" || cmd == "variable") return 0;
    return std::nullopt;
}

struct TraitState {
    std::unordered_set<std::string> param_set;
    std::unordered_map<std::string, ProcArgTraits> traits;
    std::unordered_map<std::string, std::string> upvar_aliases; // local -> param
};

void handle_upvar(const std::vector<std::string>& args,
                  TraitState& state) {
    std::size_t start = 0;
    if (!args.empty() && !args[0].empty() &&
        (std::isdigit(static_cast<unsigned char>(args[0][0])) != 0 || args[0][0] == '#'))
        start = 1;

    std::size_t i = start;
    while (i + 1 < args.size()) {
        auto other_vn = extract_var_name(args[i]);
        if (other_vn && state.param_set.count(*other_vn)) {
            state.traits[*other_vn] |= ProcArgTrait::VAR_READ;
            state.upvar_aliases[args[i + 1]] = *other_vn;
        }
        auto my_vn = extract_var_name(args[i + 1]);
        if (my_vn && state.param_set.count(*my_vn)) {
            state.traits[*my_vn] |= ProcArgTrait::VAR_WRITE;
        }
        i += 2;
    }
}

void handle_foreach(const std::vector<std::string>& args,
                    TraitState& state) {
    if (args.size() < 3) return;
    auto body_vn = extract_var_name(args.back());
    if (body_vn && state.param_set.count(*body_vn))
        state.traits[*body_vn] |= ProcArgTrait::BODY;

    std::size_t i = 0;
    auto remaining = args.size() - 1;
    while (i + 1 < remaining) {
        auto list_vn = extract_var_name(args[i + 1]);
        if (list_vn && state.param_set.count(*list_vn))
            state.traits[*list_vn] |= ProcArgTrait::LOOP_LIST;
        i += 2;
    }
}

void handle_while(const std::vector<std::string>& args,
                  TraitState& state) {
    if (args.size() < 2) return;
    auto cond_vn = extract_var_name(args[0]);
    if (cond_vn && state.param_set.count(*cond_vn))
        state.traits[*cond_vn] |= ProcArgTrait::EXPR;
    auto body_vn = extract_var_name(args[1]);
    if (body_vn && state.param_set.count(*body_vn))
        state.traits[*body_vn] |= ProcArgTrait::BODY;
}

void handle_for(const std::vector<std::string>& args,
                TraitState& state) {
    if (args.size() < 4) return;
    auto init_vn = extract_var_name(args[0]);
    if (init_vn && state.param_set.count(*init_vn))
        state.traits[*init_vn] |= ProcArgTrait::BODY;
    auto cond_vn = extract_var_name(args[1]);
    if (cond_vn && state.param_set.count(*cond_vn))
        state.traits[*cond_vn] |= ProcArgTrait::EXPR;
    auto next_vn = extract_var_name(args[2]);
    if (next_vn && state.param_set.count(*next_vn))
        state.traits[*next_vn] |= ProcArgTrait::BODY;
    auto body_vn = extract_var_name(args[3]);
    if (body_vn && state.param_set.count(*body_vn))
        state.traits[*body_vn] |= ProcArgTrait::BODY;
}

void handle_after(const std::vector<std::string>& args,
                  TraitState& state) {
    if (args.size() < 2) return;
    if (args[0] == "cancel" || args[0] == "info") return;
    std::size_t start = 1;
    if (start < args.size() && args[start] == "-periodic") ++start;
    for (std::size_t i = start; i < args.size(); ++i) {
        auto vn = extract_var_name(args[i]);
        if (vn && state.param_set.count(*vn))
            state.traits[*vn] |= ProcArgTrait::EVAL;
    }
}

void handle_variadic_var_write(const std::vector<std::string>& args,
                               TraitState& state,
                               std::size_t start) {
    for (std::size_t i = start; i < args.size(); ++i) {
        auto vn = extract_var_name(args[i]);
        if (vn && state.param_set.count(*vn))
            state.traits[*vn] |= ProcArgTrait::VAR_WRITE;
    }
}

void handle_regexp_vars(const std::vector<std::string>& args,
                        TraitState& state) {
    // Reuse skip_options to find first positional arg.
    std::vector<std::string> span_args(args.begin(), args.end());
    auto pat_idx = regexp_pattern_index(span_args);
    if (!pat_idx) return;
    // matchVar starts 2 after the pattern index.
    auto var_start = static_cast<std::size_t>(*pat_idx) + 2;
    handle_variadic_var_write(args, state, var_start);
}

void handle_regsub_var(const std::vector<std::string>& args,
                       TraitState& state) {
    std::vector<std::string> span_args(args.begin(), args.end());
    auto pat_idx = regexp_pattern_index(span_args);
    if (!pat_idx) return;
    // varName is at pattern + 3 (pattern, string, subSpec, varName).
    auto var_idx = static_cast<std::size_t>(*pat_idx) + 3;
    if (var_idx < args.size()) {
        auto vn = extract_var_name(args[var_idx]);
        if (vn && state.param_set.count(*vn))
            state.traits[*vn] |= ProcArgTrait::VAR_WRITE;
    }
}

void scan_commands(const std::vector<SegmentedCommand>& commands,
                   TraitState& state) {
    for (const auto& cmd : commands) {
        if (cmd.texts.empty()) continue;
        const auto& cmd_name = cmd.texts[0];
        auto args = std::vector<std::string>(cmd.texts.begin() + 1, cmd.texts.end());

        // eval/subst with param as script arg.
        if (cmd_name == "eval" || cmd_name == "subst") {
            for (const auto& arg : args) {
                auto vn = extract_var_name(arg);
                if (vn && state.param_set.count(*vn))
                    state.traits[*vn] |= ProcArgTrait::EVAL;
            }
        }

        // uplevel: last arg is the script.
        if (cmd_name == "uplevel" && !args.empty()) {
            auto vn = extract_var_name(args.back());
            if (vn && state.param_set.count(*vn))
                state.traits[*vn] |= ProcArgTrait::EVAL;
        }

        if (cmd_name == "upvar") handle_upvar(args, state);
        if (cmd_name == "foreach" || cmd_name == "lmap") handle_foreach(args, state);
        if (cmd_name == "while") handle_while(args, state);
        if (cmd_name == "for") handle_for(args, state);
        if (cmd_name == "after") handle_after(args, state);
        if (cmd_name == "scan") handle_variadic_var_write(args, state, 2);
        if (cmd_name == "lassign") handle_variadic_var_write(args, state, 1);
        if (cmd_name == "regexp") handle_regexp_vars(args, state);
        if (cmd_name == "regsub") handle_regsub_var(args, state);

        // binary scan: args from index 2+ are output variables.
        if (cmd_name == "binary" && args.size() >= 3 && args[0] == "scan")
            handle_variadic_var_write(args, state, 2);

        // Variable-writing commands where param is used as var name.
        auto write_idx = var_write_index(cmd_name);
        if (write_idx && *write_idx < args.size()) {
            auto vn = extract_var_name(args[*write_idx]);
            if (vn && state.param_set.count(*vn))
                state.traits[*vn] |= ProcArgTrait::VAR_WRITE;
        }

        // Track writes through upvar aliases.
        if ((cmd_name == "set" || cmd_name == "incr" ||
             cmd_name == "append" || cmd_name == "lappend") && !args.empty()) {
            auto it = state.upvar_aliases.find(args[0]);
            if (it != state.upvar_aliases.end())
                state.traits[it->second] |= ProcArgTrait::VAR_WRITE;
        }

        // foreach/lmap loop variables written through upvar aliases.
        if ((cmd_name == "foreach" || cmd_name == "lmap") && args.size() >= 3) {
            std::size_t i = 0;
            auto remaining = args.size() - 1;
            while (i < remaining) {
                auto it = state.upvar_aliases.find(args[i]);
                if (it != state.upvar_aliases.end())
                    state.traits[it->second] |= ProcArgTrait::VAR_WRITE;
                i += 2;
            }
        }
    }
}

} // namespace

auto infer_param_traits_shallow(
    std::span<const std::string> params,
    std::string_view body_source,
    const CommandRegistryInterface* /*registry*/)
    -> std::unordered_map<std::string, ProcArgTraits> {

    if (params.empty() || body_source.empty()) return {};

    // Strip whitespace check.
    bool has_content = false;
    for (char c : body_source) {
        if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
            has_content = true;
            break;
        }
    }
    if (!has_content) return {};

    TraitState state;
    for (const auto& p : params)
        state.param_set.insert(p);

    // Segment the body to get commands.
    auto commands = segment_commands(std::string(body_source));

    scan_commands(commands, state);

    // Return only params with detected traits.
    std::unordered_map<std::string, ProcArgTraits> result;
    for (auto& [name, traits] : state.traits) {
        if (traits != ProcArgTraits{0})
            result[name] = traits;
    }
    return result;
}

} // namespace tcl_lsp
