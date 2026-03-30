// Command dispatch and command-specific handlers.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/arg_role_resolver.hpp"
#include "tcl_lsp/analysis/param_list_parser.hpp"
#include "tcl_lsp/parsing/recovery.hpp"

#include <algorithm>
#include <cctype>
#include <variant>

namespace tcl_lsp {

namespace {

auto range_from_token(const Token& tok) -> Range {
    return {tok.start, tok.end};
}

} // namespace

// ---------------------------------------------------------------------------
// process_command — central dispatch
// ---------------------------------------------------------------------------

void Analyser::process_command(const SegmentedCommand& cmd, Scope* scope,
                               std::string_view /*source*/) {
    if (cmd.argv.empty()) return;

    const auto& cmd_name = cmd.texts[0];
    const auto args = cmd.args();

    // Record command invocation.
    auto* resolved_proc = find_proc_call(cmd_name, scope);
    s_.result.command_invocations_.push_back(CommandInvocation{
        cmd_name, range_from_token(cmd.argv[0]),
        resolved_proc ? resolved_proc->qualified_name : ""});

    // Package / source / dynamic provider tracking.
    handle_package(cmd);
    handle_source(cmd);

    if (cmd_name == "load") {
        s_.result.has_dynamic_providers_ = true;
    } else if ((cmd_name == "set" || cmd_name == "lappend") && !args.empty() &&
               args[0] == "auto_path") {
        s_.result.has_dynamic_providers_ = true;
    } else if (cmd_name == "rename") {
        s_.result.has_dynamic_providers_ = true;
    } else if (cmd_name == "namespace" && args.size() >= 2 && args[0] == "import") {
        s_.result.has_dynamic_providers_ = true;
    }

    // --- Consuming handlers (return early if command was fully handled) ---
    static const std::unordered_map<std::string_view, ConsumingHandler> consuming_handlers{
        {"proc",      &Analyser::handle_proc},
        {"namespace", &Analyser::handle_namespace_eval},
        {"foreach",   &Analyser::handle_foreach},
        {"for",       &Analyser::handle_for},
        {"switch",    &Analyser::handle_switch},
        {"catch",     &Analyser::handle_catch},
        {"try",       &Analyser::handle_try},
        {"if",        &Analyser::handle_if},
        {"while",     &Analyser::handle_while},
        {"dict",      &Analyser::handle_dict},
    };
    if (auto it = consuming_handlers.find(cmd_name); it != consuming_handlers.end()) {
        if ((this->*(it->second))(cmd, scope)) return;
    }

    // --- Non-consuming handlers (augment generic analysis) ---
    static const std::unordered_map<std::string_view, NonConsumingHandler> non_consuming_handlers{
        {"set",      &Analyser::handle_set},
        {"incr",     &Analyser::handle_incr},
        {"variable", &Analyser::handle_variable_decl},
        {"global",   &Analyser::handle_variable_decl},
        {"expr",     &Analyser::handle_expr},
        {"regexp",   &Analyser::handle_regexp},
        {"regsub",   &Analyser::handle_regexp},
    };
    if (auto it = non_consuming_handlers.find(cmd_name); it != non_consuming_handlers.end()) {
        (this->*(it->second))(cmd, scope);
    }
    if (cmd_name == "interp") {
        handle_interp_alias(cmd);
    }

    // array set arrayName list — defines the array variable.
    if (cmd_name == "array" && args.size() >= 2 && args[0] == "set") {
        const auto atoks = cmd.arg_tokens();
        Range r = atoks.size() >= 2 ? range_from_token(atoks[1]) : range_from_token(cmd.argv[0]);
        define_var(args[1], r, scope, false);
    }

    // --- Generic arg-role analysis via registry ---
    analyse_body_args(cmd, scope);

    // --- Arity checking ---
    // Skip arity checking when {*} expansion is used — we cannot know
    // the actual argument count at analysis time.
    bool has_expand = false;
    if (cmd.expand_word.has_value()) {
        for (bool ew : *cmd.expand_word) {
            if (ew) { has_expand = true; break; }
        }
    }
    bool has_sig = false;
    if (registry_ && !has_expand) {
        auto arity_opt = registry_->validation(cmd_name);
        if (arity_opt.has_value()) {
            has_sig = true;
            auto nargs = static_cast<int32_t>(args.size());
            if (nargs < arity_opt->min) {
                emit_diagnostic(range_from_token(cmd.argv[0]), Severity::ERROR, DiagCode::E001,
                                "Too few arguments for '" + cmd_name + "'");
            } else if (nargs > arity_opt->max) {
                emit_diagnostic(range_from_token(cmd.argv[0]), Severity::ERROR, DiagCode::E001,
                                "Too many arguments for '" + cmd_name + "'");
            }
        }
        // Subcommand-level arity check.
        if (!has_sig) {
            auto sig_opt = registry_->signature(cmd_name);
            if (sig_opt.has_value()) {
                if (auto* ss = std::get_if<SubcommandSig>(&*sig_opt)) {
                    has_sig = true;
                    if (!args.empty()) {
                        auto it = ss->subcommands.find(args[0]);
                        if (it != ss->subcommands.end()) {
                            // Args to the subcommand (excluding subcommand name itself).
                            auto sub_nargs = static_cast<int32_t>(args.size()) - 1;
                            if (sub_nargs < it->second.arity.min) {
                                emit_diagnostic(range_from_token(cmd.argv[0]), Severity::ERROR,
                                                DiagCode::E001,
                                                "Too few arguments for '" + cmd_name +
                                                    " " + args[0] + "'");
                            } else if (sub_nargs > it->second.arity.max) {
                                emit_diagnostic(range_from_token(cmd.argv[0]), Severity::ERROR,
                                                DiagCode::E001,
                                                "Too many arguments for '" + cmd_name +
                                                    " " + args[0] + "'");
                            }
                        }
                    }
                }
            }
        }
        if (!has_sig) {
            auto [role_cmd, role_args] = s_.alias_resolver.resolve(
                cmd_name, args, scope ? namespace_from_scope(scope) : "::");
            if (role_cmd != cmd_name) {
                auto alias_arity = registry_->validation(role_cmd);
                if (alias_arity.has_value()) {
                    has_sig = true;
                    // Effective arg count = prepended alias args + caller's args.
                    auto nargs = static_cast<int32_t>(role_args.size());
                    if (nargs < alias_arity->min) {
                        emit_diagnostic(range_from_token(cmd.argv[0]), Severity::ERROR,
                                        DiagCode::E001,
                                        "Too few arguments for '" + cmd_name + "'");
                    } else if (nargs > alias_arity->max) {
                        emit_diagnostic(range_from_token(cmd.argv[0]), Severity::ERROR,
                                        DiagCode::E001,
                                        "Too many arguments for '" + cmd_name + "'");
                    }
                } else {
                    has_sig = registry_->signature(role_cmd).has_value();
                }
            }
        }
    }
    if (!has_sig && resolved_proc) {
        check_proc_call_arity(*resolved_proc, args, cmd.argv[0]);
    }
}

// ---------------------------------------------------------------------------
// handle_proc
// ---------------------------------------------------------------------------

auto Analyser::handle_proc(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 3) return false;
    const auto atoks = cmd.arg_tokens();

    const auto& proc_name = args[0];
    const auto& param_str = args.size() > 1 ? args[1] : "";
    const auto& body = args.size() > 2 ? args[2] : "";

    auto params = parse_param_list(param_str);

    // Build qualified name.
    std::string qualified;
    auto ns = namespace_from_scope(scope);
    if (ns == "::") {
        qualified = "::" + proc_name;
    } else {
        qualified = ns + "::" + proc_name;
    }
    qualified = normalise_qualified_name(qualified, nullptr);

    if (atoks.empty()) return true;
    auto name_range = range_from_token(atoks[0]);
    auto body_range = atoks.size() > 2 ? range_from_token(atoks[2]) : name_range;

    // W113: proc shadows built-in command.
    if (registry_) {
        auto normalised_proc = proc_name;
        while (!normalised_proc.empty() && normalised_proc[0] == ':') {
            normalised_proc.erase(normalised_proc.begin());
        }
        auto normalised_qual = qualified;
        while (!normalised_qual.empty() && normalised_qual[0] == ':') {
            normalised_qual.erase(normalised_qual.begin());
        }
        std::string shadow_name;
        if (registry_->is_known(normalised_proc)) {
            shadow_name = normalised_proc;
        } else if (registry_->is_known(normalised_qual)) {
            shadow_name = normalised_qual;
        }
        if (!shadow_name.empty()) {
            emit_diagnostic(name_range, Severity::WARNING, DiagCode::W113,
                            "Procedure '" + proc_name + "' shadows built-in command");
        }
    }

    // Extract doc from preceding comment.
    auto doc = s_.last_comment;
    s_.last_comment.clear();

    ProcDef proc_def;
    proc_def.name = proc_name;
    proc_def.qualified_name = qualified;
    proc_def.params = params;
    proc_def.name_range = name_range;
    proc_def.body_range = body_range;
    proc_def.doc = doc;

    scope->procs[proc_name] = proc_def;
    s_.result.all_procs_[qualified] = &scope->procs[proc_name];

    // Create proc child scope.
    auto* proc_scope = make_child_scope(ScopeKind::PROC, proc_name, scope);
    proc_scope->body_range = body_range;

    // Define parameters as variables.
    for (const auto& p : params) {
        proc_scope->variables[p.name] =
            VarDef{p.name, name_range, {}, false};
    }

    // Analyse body.
    auto saved_comment = s_.last_comment;
    const Token* body_tok = atoks.size() > 2 ? &atoks[2] : nullptr;
    analyse_body(body, proc_scope, body_tok);
    s_.last_comment = saved_comment;

    // W214: unused proc parameters.
    for (const auto& p : params) {
        if (p.name == "args") continue;
        if (!p.name.empty() && p.name[0] == '_') continue;
        auto it = proc_scope->variables.find(p.name);
        if (it != proc_scope->variables.end() && it->second.references.empty()) {
            emit_diagnostic(name_range, Severity::HINT, DiagCode::W214,
                            "Parameter '" + p.name + "' of '" + proc_name +
                                "' is unused");
        }
    }

    // Detect 'unknown' proc.
    auto norm_qual = qualified;
    while (!norm_qual.empty() && norm_qual[0] == ':') {
        norm_qual.erase(norm_qual.begin());
    }
    if (proc_name == "unknown" || norm_qual == "unknown" ||
        norm_qual == "tcl::unknown") {
        extract_unknown_proc_info(body);
    }
    return true;
}

// ---------------------------------------------------------------------------
// handle_set
// ---------------------------------------------------------------------------

void Analyser::handle_set(const SegmentedCommand& cmd, Scope* scope) {
    const auto args = cmd.args();
    const auto atoks = cmd.arg_tokens();
    if (args.empty() || atoks.empty()) return;

    if (args.size() >= 2) {
        // Assignment: define variable.
        define_var(args[0], range_from_token(atoks[0]), scope, true);

        // Track constant string values for regex propagation.
        if (atoks.size() >= 2) {
            const auto& vtok = atoks[1];
            if ((vtok.type == TokenType::ESC || vtok.type == TokenType::STR) &&
                vtok.text == args[1]) {
                set_const_string(args[0], args[1], range_from_token(vtok), scope);
            } else {
                clear_const_string(args[0], scope);
            }
        }
    } else {
        // Read-only form: record reference.
        record_var_read(args[0], range_from_token(atoks[0]), scope);
    }
}

// ---------------------------------------------------------------------------
// handle_variable_decl (variable / global)
// ---------------------------------------------------------------------------

void Analyser::handle_variable_decl(const SegmentedCommand& cmd, Scope* scope) {
    const auto& cmd_name = cmd.texts[0];
    const auto args = cmd.args();
    const auto atoks = cmd.arg_tokens();

    if (cmd_name == "global") {
        for (std::size_t i = 0; i < args.size(); ++i) {
            if (i < atoks.size()) {
                define_var(args[i], range_from_token(atoks[i]), scope, false);
            }
        }
        return;
    }

    // 'variable' — alternating name ?value? pairs.
    std::size_t i = 0;
    while (i < args.size()) {
        if (i < atoks.size()) {
            define_var(args[i], range_from_token(atoks[i]), scope, false);
        }
        i += (i + 1 < args.size()) ? 2 : 1;
    }
}

// ---------------------------------------------------------------------------
// handle_namespace_eval
// ---------------------------------------------------------------------------

auto Analyser::handle_namespace_eval(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 2 || args[0] != "eval") return false;
    const auto atoks = cmd.arg_tokens();
    // args[0]="eval", args[1]=ns_name, args[2]=body
    const auto& ns_name = args[1];

    auto* ns_scope = make_child_scope(ScopeKind::NAMESPACE, ns_name, scope);
    if (atoks.size() > 2) {
        ns_scope->body_range = range_from_token(atoks[2]);
    }

    if (args.size() >= 3) {
        const Token* btok = atoks.size() > 2 ? &atoks[2] : nullptr;
        analyse_body(args[2], ns_scope, btok);
    }
    return true;
}

// ---------------------------------------------------------------------------
// handle_foreach
// ---------------------------------------------------------------------------

auto Analyser::handle_foreach(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 3) return false;
    const auto atoks = cmd.arg_tokens();

    if (!atoks.empty()) {
        define_vars_from_list(args[0], atoks[0], scope);
    }

    const Token* btok = !atoks.empty() ? &atoks.back() : nullptr;
    analyse_body(args.back(), scope, btok);
    return true;
}

// ---------------------------------------------------------------------------
// handle_for
// ---------------------------------------------------------------------------

auto Analyser::handle_for(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 4) return false;
    const auto atoks = cmd.arg_tokens();

    const Token* init_tok = !atoks.empty() ? &atoks[0] : nullptr;
    analyse_body(args[0], scope, init_tok);

    if (args.size() > 1) {
        analyse_expr(args[1], scope);
    }

    if (args.size() > 2) {
        const Token* next_tok = atoks.size() > 2 ? &atoks[2] : nullptr;
        analyse_body(args[2], scope, next_tok);
    }

    if (args.size() > 3) {
        const Token* body_tok = atoks.size() > 3 ? &atoks[3] : nullptr;
        analyse_body(args[3], scope, body_tok);
    }
    return true;
}

// ---------------------------------------------------------------------------
// handle_switch
// ---------------------------------------------------------------------------

auto Analyser::handle_switch(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 2) return false;
    const auto atoks = cmd.arg_tokens();

    // Parse options to detect -regexp.
    bool is_regexp = false;
    std::size_t i = 0;
    while (i < args.size() && !args[i].empty() && args[i][0] == '-') {
        if (args[i] == "-regexp") is_regexp = true;
        if (args[i] == "--") { ++i; break; }
        ++i;
    }
    // Skip the string argument.
    ++i;

    if (i < args.size() && i == args.size() - 1) {
        // Form 2: single braced body.
        const Token* btok = i < atoks.size() ? &atoks[i] : nullptr;
        parse_switch_body(args[i], btok, scope, is_regexp);
    } else {
        // Form 1: inline pattern/body pairs.
        while (i + 1 < args.size()) {
            if (is_regexp && args[i] != "default" && i < atoks.size()) {
                const auto& pat_tok = atoks[i];
                if (pat_tok.type == TokenType::VAR) {
                    auto cv = lookup_const_string(pat_tok.text, scope);
                    if (cv.has_value()) {
                        s_.result.regex_patterns_.push_back(
                            RegexPattern{range_from_token(pat_tok), cv->first, "switch"});
                        scope->regex_vars.insert(pat_tok.text);
                        record_defining_set_as_regex(pat_tok.text, scope, "switch");
                    }
                } else {
                    s_.result.regex_patterns_.push_back(
                        RegexPattern{range_from_token(pat_tok), args[i], "switch"});
                }
            }
            if (args[i + 1] != "-") {
                const Token* btok = (i + 1) < atoks.size() ? &atoks[i + 1] : nullptr;
                analyse_body(args[i + 1], scope, btok);
            }
            i += 2;
        }
    }
    return true;
}

void Analyser::parse_switch_body(std::string_view body_text, const Token* body_token,
                                 Scope* scope, bool is_regexp) {
    // Re-lex the braced body to extract pattern/body pair elements.
    auto [inner_cmds, inner_diags] = segment_with_recovery(body_text, body_token);

    // Propagate recovery diagnostics from the braced body.
    for (auto& diag : inner_diags) {
        s_.result.diagnostics_.push_back(std::move(diag));
    }

    // Collect element texts and tokens from the inner parse.
    std::vector<std::string> elements;
    std::vector<Token> elem_tokens;
    for (const auto& icmd : inner_cmds) {
        for (std::size_t j = 0; j < icmd.texts.size(); ++j) {
            elements.push_back(icmd.texts[j]);
            elem_tokens.push_back(j < icmd.argv.size() ? icmd.argv[j] : Token{});
        }
    }

    // Process alternating pattern/body pairs.
    std::size_t i = 0;
    while (i + 1 < elements.size()) {
        if (is_regexp && elements[i] != "default" && i < elem_tokens.size()) {
            const auto& pat_tok = elem_tokens[i];
            if (pat_tok.type == TokenType::VAR) {
                auto cv = lookup_const_string(pat_tok.text, scope);
                if (cv.has_value()) {
                    s_.result.regex_patterns_.push_back(
                        RegexPattern{range_from_token(pat_tok), cv->first, "switch"});
                    scope->regex_vars.insert(pat_tok.text);
                    record_defining_set_as_regex(pat_tok.text, scope, "switch");
                }
            } else {
                s_.result.regex_patterns_.push_back(
                    RegexPattern{range_from_token(pat_tok), elements[i], "switch"});
            }
        }
        if (elements[i + 1] != "-") {
            const Token* btok = (i + 1) < elem_tokens.size() ? &elem_tokens[i + 1] : nullptr;
            analyse_body(elements[i + 1], scope, btok);
        }
        i += 2;
    }
}

// ---------------------------------------------------------------------------
// handle_catch
// ---------------------------------------------------------------------------

auto Analyser::handle_catch(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.empty()) return false;
    const auto atoks = cmd.arg_tokens();

    const Token* body_tok = !atoks.empty() ? &atoks[0] : nullptr;
    s_.conditional_depth++;
    analyse_body(args[0], scope, body_tok);
    s_.conditional_depth--;

    // Optional result/options variables (indices 1, 2).
    for (std::size_t i = 1; i < std::min(args.size(), std::size_t{3}); ++i) {
        if (i < atoks.size()) {
            define_var(args[i], range_from_token(atoks[i]), scope, false);
        }
    }
    return true;
}

// ---------------------------------------------------------------------------
// handle_try
// ---------------------------------------------------------------------------

auto Analyser::handle_try(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.empty()) return false;
    const auto atoks = cmd.arg_tokens();

    // First arg is the try body.
    const Token* body_tok = !atoks.empty() ? &atoks[0] : nullptr;
    analyse_body(args[0], scope, body_tok);

    // Scan for on/trap/finally handlers.
    std::size_t i = 1;
    while (i < args.size()) {
        if (args[i] == "finally" && i + 1 < args.size()) {
            const Token* ftok = (i + 1) < atoks.size() ? &atoks[i + 1] : nullptr;
            analyse_body(args[i + 1], scope, ftok);
            i += 2;
        } else if ((args[i] == "on" || args[i] == "trap") && i + 3 < args.size()) {
            // args[i+2] is the variable list (e.g. {msg opts}).
            auto var_list = parse_param_list(args[i + 2]);
            for (const auto& p : var_list) {
                Range vr = (i + 2) < atoks.size() ? range_from_token(atoks[i + 2])
                                                  : Range::zero();
                define_var(p.name, vr, scope, false);
            }
            const Token* htok = (i + 3) < atoks.size() ? &atoks[i + 3] : nullptr;
            analyse_body(args[i + 3], scope, htok);
            i += 4;
        } else {
            ++i;
        }
    }
    return true;
}

// ---------------------------------------------------------------------------
// handle_incr
// ---------------------------------------------------------------------------

void Analyser::handle_incr(const SegmentedCommand& cmd, Scope* scope) {
    const auto args = cmd.args();
    const auto atoks = cmd.arg_tokens();
    if (!args.empty() && !atoks.empty()) {
        define_var(args[0], range_from_token(atoks[0]), scope, true);
    }
}

// ---------------------------------------------------------------------------
// handle_if
// ---------------------------------------------------------------------------

auto Analyser::handle_if(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 2) return false;
    const auto atoks = cmd.arg_tokens();

    s_.conditional_depth++;

    // if expr ?then? body ?elseif expr ?then? body?... ?else bodyN?
    std::size_t i = 0;

    // First condition + body.
    if (i < args.size()) {
        analyse_expr(args[i], scope);
        ++i;
    }
    if (i < args.size() && args[i] == "then") ++i;
    if (i < args.size()) {
        const Token* btok = i < atoks.size() ? &atoks[i] : nullptr;
        analyse_body(args[i], scope, btok);
        ++i;
    }

    // Subsequent elseif/else clauses.
    while (i < args.size()) {
        if (args[i] == "elseif") {
            ++i;
            if (i < args.size()) { analyse_expr(args[i], scope); ++i; }
            if (i < args.size() && args[i] == "then") ++i;
            if (i < args.size()) {
                const Token* btok = i < atoks.size() ? &atoks[i] : nullptr;
                analyse_body(args[i], scope, btok);
                ++i;
            }
        } else if (args[i] == "else") {
            ++i;
            if (i < args.size()) {
                const Token* btok = i < atoks.size() ? &atoks[i] : nullptr;
                analyse_body(args[i], scope, btok);
                ++i;
            }
        } else {
            ++i;
        }
    }

    s_.conditional_depth--;
    return true;
}

// ---------------------------------------------------------------------------
// handle_while
// ---------------------------------------------------------------------------

auto Analyser::handle_while(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 2) return false;
    const auto atoks = cmd.arg_tokens();

    analyse_expr(args[0], scope);

    const Token* btok = atoks.size() > 1 ? &atoks[1] : nullptr;
    analyse_body(args[1], scope, btok);
    return true;
}

// ---------------------------------------------------------------------------
// handle_dict
// ---------------------------------------------------------------------------

auto Analyser::handle_dict(const SegmentedCommand& cmd, Scope* scope) -> bool {
    const auto args = cmd.args();
    if (args.size() < 4 || args[0] != "for") return false;
    const auto atoks = cmd.arg_tokens();

    define_vars_from_list(args[1], atoks.size() > 1 ? atoks[1] : cmd.argv[0], scope);
    const Token* btok = atoks.size() > 3 ? &atoks[3] : nullptr;
    analyse_body(args[3], scope, btok);
    return true;
}

// ---------------------------------------------------------------------------
// handle_expr
// ---------------------------------------------------------------------------

void Analyser::handle_expr(const SegmentedCommand& cmd, Scope* scope) {
    const auto args = cmd.args();
    if (args.empty()) return;
    for (const auto& arg : args) {
        analyse_expr(arg, scope);
    }
}

// ---------------------------------------------------------------------------
// handle_regexp — option-aware pattern detection for regexp/regsub
// ---------------------------------------------------------------------------

void Analyser::handle_regexp(const SegmentedCommand& cmd, Scope* scope) {
    const auto& cmd_name = cmd.texts[0];
    const auto args = cmd.args();
    const auto atoks = cmd.arg_tokens();

    auto pattern_indices = arg_indices_for_role(registry_, cmd_name, args, ArgRole::PATTERN);
    for (auto idx : pattern_indices) {
        auto uidx = static_cast<std::size_t>(idx);
        if (uidx >= atoks.size()) continue;
        const auto& pt = atoks[uidx];
        if (pt.type == TokenType::VAR) {
            auto cv = lookup_const_string(pt.text, scope);
            if (cv.has_value()) {
                s_.result.regex_patterns_.push_back(
                    RegexPattern{range_from_token(pt), cv->first, cmd_name});
                scope->regex_vars.insert(pt.text);
                record_defining_set_as_regex(pt.text, scope, cmd_name);
            }
        } else {
            s_.result.regex_patterns_.push_back(
                RegexPattern{range_from_token(pt), args[uidx], cmd_name});
        }
    }
}

// ---------------------------------------------------------------------------
// handle_interp_alias
// ---------------------------------------------------------------------------

void Analyser::handle_interp_alias(const SegmentedCommand& cmd) {
    const auto args = cmd.args();
    // interp alias {} srcToken {} targetCmd ?arg ...?
    if (args.size() < 5 || args[0] != "alias") return;

    const auto& src_path = args[1];
    const auto& alias_name = args[2];
    const auto& target_path = args[3];
    const auto& target_cmd = args[4];

    if ((src_path != "" && src_path != "{}") ||
        (target_path != "" && target_path != "{}")) {
        return;
    }

    auto qualified = normalise_qualified_name(alias_name, nullptr);
    std::vector<std::string> prepended(args.begin() + 5, args.end());

    s_.alias_resolver.register_alias(qualified, target_cmd, prepended);
    s_.result.command_aliases_[qualified] = {target_cmd, std::vector<std::string>(prepended)};
}

// ---------------------------------------------------------------------------
// handle_package
// ---------------------------------------------------------------------------

void Analyser::handle_package(const SegmentedCommand& cmd) {
    const auto args = cmd.args();
    if (cmd.texts[0] != "package" || args.empty()) return;

    if (args[0] == "require" && args.size() >= 2) {
        std::string pkg_name = args[1];
        std::string pkg_version;
        if (pkg_name == "-exact" && args.size() >= 3) {
            pkg_name = args[2];
            pkg_version = args.size() >= 4 ? args[3] : "";
        } else {
            pkg_version = args.size() >= 3 ? args[2] : "";
        }
        // Dynamic package names (variable/command substitution) mean we can't
        // know which packages are loaded at analysis time.
        if (pkg_name.find('$') != std::string::npos ||
            pkg_name.find('[') != std::string::npos) {
            s_.result.has_dynamic_providers_ = true;
        }
        s_.result.package_requires_.push_back(PackageRequire{
            pkg_name, pkg_version, range_from_token(cmd.argv[0]),
            s_.conditional_depth > 0});
    } else if (args[0] == "provide" && args.size() >= 2) {
        auto pkg_version = args.size() >= 3 ? args[2] : std::string{};
        s_.result.package_provides_.push_back(PackageProvide{
            args[1], pkg_version, range_from_token(cmd.argv[0])});
    }
}

// ---------------------------------------------------------------------------
// handle_source
// ---------------------------------------------------------------------------

void Analyser::handle_source(const SegmentedCommand& cmd) {
    const auto args = cmd.args();
    const auto atoks = cmd.arg_tokens();
    if (cmd.texts[0] != "source" || args.empty()) return;

    std::size_t file_idx = (args[0] == "-encoding" && args.size() >= 3) ? 2 : 0;
    if (file_idx >= args.size() || file_idx >= atoks.size()) return;

    const auto& st = atoks[file_idx];
    const auto& path = args[file_idx];
    bool is_lit = st.type != TokenType::VAR && st.type != TokenType::CMD &&
                  path.find('$') == std::string::npos &&
                  path.find('[') == std::string::npos;

    s_.result.source_targets_.push_back(SourceTarget{path, range_from_token(st), is_lit});
}

// ---------------------------------------------------------------------------
// analyse_body_args — generic arg-role analysis via registry
// ---------------------------------------------------------------------------

void Analyser::analyse_body_args(const SegmentedCommand& cmd, Scope* scope) {
    if (!registry_) return;

    const auto& cmd_name = cmd.texts[0];
    const auto args = cmd.args();
    const auto atoks = cmd.arg_tokens();

    auto [role_cmd, role_args] = s_.alias_resolver.resolve(
                cmd_name, args, scope ? namespace_from_scope(scope) : "::");
    auto prepend_n = static_cast<int32_t>(role_args.size()) -
                     static_cast<int32_t>(args.size());

    auto sig_opt = registry_->signature(role_cmd);
    if (!sig_opt.has_value() && role_cmd != cmd_name) {
        sig_opt = registry_->signature(cmd_name);
    }
    if (!sig_opt.has_value()) return;

    const CommandSig* csig = nullptr;
    CommandSig resolved_csig;
    int32_t sub_offset = 0;

    if (auto* cs = std::get_if<CommandSig>(&*sig_opt)) {
        csig = cs;
    } else if (auto* ss = std::get_if<SubcommandSig>(&*sig_opt)) {
        if (!role_args.empty()) {
            auto it = ss->subcommands.find(role_args[0]);
            if (it != ss->subcommands.end()) {
                resolved_csig = it->second;
                csig = &resolved_csig;
                sub_offset = 1;
            }
        }
    }
    if (!csig) return;

    for (const auto& [virtual_idx, role] : csig->arg_roles) {
        auto idx = virtual_idx + sub_offset - prepend_n;
        if (idx < 0 || idx >= static_cast<int32_t>(args.size())) continue;
        auto uidx = static_cast<std::size_t>(idx);

        switch (role) {
            case ArgRole::BODY: {
                const Token* btok = uidx < atoks.size() ? &atoks[uidx] : nullptr;
                analyse_body(args[uidx], scope, btok);
                break;
            }
            case ArgRole::EXPR:
                analyse_expr(args[uidx], scope);
                break;
            case ArgRole::VAR_NAME:
                if (cmd_name != "set" && cmd_name != "variable" &&
                    cmd_name != "global" && cmd_name != "incr") {
                    Range r = uidx < atoks.size() ? range_from_token(atoks[uidx])
                                                  : range_from_token(cmd.argv[0]);
                    define_var(args[uidx], r, scope, false);
                }
                break;
            case ArgRole::PATTERN:
                // regexp/regsub are handled by handle_regexp with proper
                // option-aware index resolution. Skip here to avoid
                // double-counting or using incorrect FIXED indices.
                if (role_cmd == "regexp" || role_cmd == "regsub") break;
                if (uidx < atoks.size()) {
                    const auto& pt = atoks[uidx];
                    if (pt.type == TokenType::VAR) {
                        auto cv = lookup_const_string(pt.text, scope);
                        if (cv.has_value()) {
                            s_.result.regex_patterns_.push_back(
                                RegexPattern{range_from_token(pt), cv->first, role_cmd});
                            scope->regex_vars.insert(pt.text);
                            record_defining_set_as_regex(pt.text, scope, role_cmd);
                        }
                    } else {
                        s_.result.regex_patterns_.push_back(
                            RegexPattern{range_from_token(pt), args[uidx], role_cmd});
                    }
                }
                break;
            default:
                break;
        }
    }
}

// AliasResolver methods are implemented in analyser_helpers.cpp (needs normalise_qualified).

// ---------------------------------------------------------------------------
// find_proc_call
// ---------------------------------------------------------------------------

auto Analyser::find_proc_call(const std::string& cmd_name, Scope* scope) -> ProcDef* {
    if (cmd_name.empty()) return nullptr;

    // Build candidate qualified names.
    auto try_find = [&](const std::string& raw) -> ProcDef* {
        auto q = normalise_qualified_name(raw, nullptr);
        if (q.empty()) return nullptr;
        auto it = s_.result.all_procs_.find(q);
        return it != s_.result.all_procs_.end() ? it->second : nullptr;
    };

    if (cmd_name.starts_with("::")) {
        return try_find(cmd_name);
    }
    if (cmd_name.find("::") != std::string::npos) {
        return try_find("::" + cmd_name);
    }

    // Walk scope chain looking for namespace context.
    auto* s = scope;
    while (s != nullptr) {
        if (s->kind == ScopeKind::NAMESPACE) {
            if (auto* p = try_find(s->name + "::" + cmd_name)) return p;
        }
        s = s->parent;
    }
    return try_find("::" + cmd_name);
}

// ---------------------------------------------------------------------------
// check_proc_call_arity
// ---------------------------------------------------------------------------

void Analyser::check_proc_call_arity(const ProcDef& proc_def,
                                     std::span<const std::string> args,
                                     const Token& cmd_token) {
    int32_t required = 0;
    bool variadic = false;

    for (std::size_t i = 0; i < proc_def.params.size(); ++i) {
        const auto& param = proc_def.params[i];
        if (i == proc_def.params.size() - 1 && param.name == "args" && !param.default_value) {
            variadic = true;
            continue;
        }
        if (!param.default_value) {
            ++required;
        }
    }

    auto max_args = variadic ? std::numeric_limits<int32_t>::max()
                             : static_cast<int32_t>(proc_def.params.size());
    auto nargs = static_cast<int32_t>(args.size());

    if (nargs < required) {
        emit_diagnostic(range_from_token(cmd_token), Severity::ERROR, DiagCode::E002,
                        "Too few arguments for '" + proc_def.qualified_name +
                            "': expected at least " + std::to_string(required) +
                            ", got " + std::to_string(nargs));
    } else if (nargs > max_args) {
        emit_diagnostic(range_from_token(cmd_token), Severity::ERROR, DiagCode::E003,
                        "Too many arguments for '" + proc_def.qualified_name +
                            "': expected at most " + std::to_string(max_args) +
                            ", got " + std::to_string(nargs));
    }
}

} // namespace tcl_lsp
