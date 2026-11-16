// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One warning per tainted use reaching a classified sink: mitigating colours, safe positions, argument spans and the message.

use super::classification::{classify_network_interp_sinks, selected_sink_schema};
use super::sink_wrappers::{
    list_wrapped_arg_command_is_literal, var_consumed_by_sanitiser, var_wrapper_colours,
};
use super::source_reads::{operand_read_taint, original_sink_argument, reference_root_with_config};
use super::{
    ArgRole, CommandRegistry, DiagCode, SinkCall, Span, TaintColour, TaintLattice, TaintScan,
    TaintWarning, Traits, WordExpr, arg_var_names_with_config, reg_colour,
};

/// Return `true` when a tainted value `lat` carries a colour that mitigates
/// the given output-sink code.
///
/// T101 shares IRULE3002 / IRULE3003's mask because the three are one hazard
/// under three names: what an attacker injects into `puts`, a header, or a log
/// line is a CR/LF that forges a record boundary, so a value proven CR/LF-free
/// cannot commit it.
///
/// For IRULE3002 in the name-position (arg-index 1 of
/// `HTTP::header`/`HTTP::cookie` `insert`/`replace`), the
/// `HEADER_TOKEN_SAFE` colour is an additional mitigation. That extra
/// check is handled at the call site because it needs the per-use arg
/// index; the function signature here is deliberately kept narrow.
pub(in super::super) fn sink_colour_mitigated(code: DiagCode, lat: TaintLattice) -> bool {
    if !lat.is_tainted() {
        return false;
    }
    match code {
        DiagCode::Irule3001 => lat.colours.intersects(TaintColour::HTML_ESCAPED),
        DiagCode::T101 | DiagCode::Irule3002 | DiagCode::Irule3003 => {
            lat.colours.intersects(TaintColour::CRLF_SAFE)
        }
        DiagCode::Irule3004 => lat.colours.intersects(TaintColour::REDIRECT_SAFE),
        _ => false,
    }
}

/// Return `true` when the tainted var `var_name` occupies a
/// header/cookie *name* position (arg index 1 after the `insert` /
/// `replace` subcommand) in `args` and carries the
/// `HEADER_TOKEN_SAFE` colour — the IRULE3002 extra mitigation.
pub(in super::super) fn irule3002_name_position_safe(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    var_name: &str,
    lat: TaintLattice,
    original: Option<&WordExpr>,
    config: tcl_lexer::LexerConfig,
) -> bool {
    if !lat.colours.contains(TaintColour::HEADER_TOKEN_SAFE) {
        return false;
    }
    // Registry-driven + prefix-aware: an output-sink subcommand
    // (`taint_output_sink_subcommands`, e.g. `insert`/`replace`) resolved from
    // whatever the caller wrote (`ins` ⇒ `insert`), so the name-position
    // safety check matches the same set the sink classifier uses.
    let Some(spec) = registry.get(command) else {
        return false;
    };
    let Some(sub_word) = args.first() else {
        return false;
    };
    let is_output_sink_sub = !spec.taint_output_sink_subcommands.is_empty()
        && spec
            .resolve_subcommand(sub_word)
            .is_some_and(|sub| spec.taint_output_sink_subcommands.contains(&sub.name));
    if !is_output_sink_sub {
        return false;
    }
    let Some(arg) = args.get(1) else { return false };
    let Some(original) = original else {
        return false;
    };
    let Some((reference, _)) = original.sole_variable_substitution() else {
        return false;
    };
    // The current operand must still correspond to the call's argument data.
    reference == arg.trim() && reference_root_with_config(reference, config) == Some(var_name)
}

/// The tight span of the argument word that carries `name`, using the
/// statement's per-word token spans (`call.tokens.argv`, offset by one to
/// skip the command-name slot at index 0 — `call.args` excludes the command
/// name, `argv` does not). Falls back to `fallback` (the whole-statement
/// span) when no token spans are available, or `name` isn't found in any
/// argument word (e.g. it reached the sink through an already-resolved
/// alias with no textual match). Narrows the T100/T101-family highlight
/// from "the whole sink statement" down to "the one argument word that
/// carries the tainted value" — e.g. `eval "prefix" $x "suffix"` underlines
/// only `$x`.
pub(in super::super) fn sink_arg_span(call: &SinkCall<'_>, name: &str, fallback: Span) -> Span {
    let Some(tokens) = call.tokens else {
        return fallback;
    };
    let effective = crate::registry_invocation::effective_command_words(tokens);
    for (i, arg) in call.args.iter().enumerate() {
        if arg_var_names_with_config(arg, call.source.config).may_contain(name)
            && let Some(written) = effective
                .as_ref()
                .and_then(|words| words.written_argument(i))
            && let Some(&arg_span) = tokens.argv.get(written + 1)
        {
            return arg_span;
        }
    }
    fallback
}

/// Positional argument strings of `args` under `spec`, skipping option
/// flags (`-foo`) and the value of any option whose [`OptionSpec`] declares
/// `takes_value`. `--` ends option processing; everything after is
/// positional.
fn positional_arg_strings(spec: &tcl_registry::CommandSpec, args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a.len() > 1 && a.starts_with('-') {
            if a == "--" {
                out.extend(args[i + 1..].iter().cloned());
                break;
            }
            // Skip the option and the value word(s) it consumes (arity-aware).
            let consumed = spec
                .options
                .iter()
                .find(|o| o.matches(a))
                .map_or(0, |o| o.value_word_count(args, i));
            i += 1 + consumed;
            continue;
        }
        out.push(a.clone());
        i += 1;
    }
    out
}

/// Position-aware sink filter: `true` when a tainted variable `name`
/// occupies a *non-dangerous* argument slot for `code` and so must not trip
/// the sink.
pub(in super::super) fn sink_var_position_safe(
    call: &SinkCall<'_>,
    code: DiagCode,
    name: &str,
) -> bool {
    let config = call.source.config;
    if call
        .args
        .iter()
        .any(|arg| arg_var_names_with_config(arg, config).has_unknown_residual())
    {
        return false;
    }
    if code == DiagCode::T105
        && let Some(scripts) = call
            .source_read
            .and_then(|read| read.script_arguments(call))
    {
        // A path/selector operand does not become script text. This projects
        // only the same current compiler or Direct source sink candidate's roles;
        // unknown selection and missing source ownership retain the warning.
        return !call.args.iter().enumerate().any(|(index, arg)| {
            scripts.contains(&(call.argument_offset + index))
                && arg_var_names_with_config(arg, config).may_contain(name)
        });
    }
    let Some((spec, command, selected_args)) = selected_sink_schema(call) else {
        return false;
    };
    let (registry, args, config) = (call.registry, selected_args.as_slice(), call.source.config);
    if args
        .iter()
        .any(|arg| arg_var_names_with_config(arg, config).has_unknown_residual())
    {
        return false;
    }
    // A value that only reaches an `ArgRole::Channel` argument slot (e.g.
    // `puts`'s optional leading `channelId`) is a handle, not sink content
    // — regardless of which sink code classified the call. The position
    // comes from the command's registry-declared arg roles
    // (`arg_role_resolver` for `puts`, since the channel slot shifts with
    // the optional leading `-nonewline`), so a future output sink with its
    // own channel argument is covered for free with no name check here.
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let channel_positions = registry.arg_indices_for_role(&command, &arg_refs, ArgRole::Channel);
    if !channel_positions.is_empty()
        && args.iter().enumerate().all(|(i, a)| {
            channel_positions.contains(&i)
                || !arg_var_names_with_config(a, config).may_contain(name)
        })
    {
        return true;
    }
    match code {
        // T104 SSRF — only the network-address positional slots named by
        // `taint_network_sink_args`. `None`/`Some(&[])` (positions
        // unspecified) imposes no filter.
        DiagCode::T104 => {
            var_only_in_safe_positions(spec, spec.taint_network_sink_args, args, name, config)
        }
        // T100 code injection — only the code-execution slot named by
        // `taint_code_sink_args` (`apply`'s lambda `func`, `open`'s
        // pipeline-selecting `fileName` — both position 0). A tainted value
        // in any other positional slot is ordinary data (the lambda's bound
        // parameters, `open`'s access/permissions) and must not trip the
        // sink. `None` (`eval`/`uplevel`/`subst`/`exec` — the whole tail is
        // one script) or `Some(&[])` imposes no filter, so every tainted
        // argument stays dangerous as before.
        DiagCode::T100 => {
            var_only_in_safe_positions(spec, spec.taint_code_sink_args, args, name, config)
        }
        _ => false,
    }
}

/// `true` when `name` appears in `args` only *outside* the dangerous
/// positional slots named by `positions` (option flags skipped) — i.e. the
/// value never reaches a dangerous slot, so the sink must not fire.
/// `None` or `Some(&[])` means "positions unspecified": no filter, so this
/// returns `false` (the value is treated as potentially dangerous). Shared
/// by the T100 (`taint_code_sink_args`) and T104 (`taint_network_sink_args`)
/// position filters in [`sink_var_position_safe`].
fn var_only_in_safe_positions(
    spec: &tcl_registry::CommandSpec,
    positions: Option<&'static [u8]>,
    args: &[String],
    name: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Some(positions) = positions else {
        return false;
    };
    if positions.is_empty() {
        return false;
    }
    let positionals = positional_arg_strings(spec, args);
    let in_dangerous_slot = positions.iter().any(|&p| {
        positionals
            .get(p as usize)
            .is_some_and(|s| arg_var_names_with_config(s, config).may_contain(name))
    });
    !in_dangerous_slot
}

/// The message a classified sink emits for one tainted use.
///
/// Kept apart from [`emit_sink_warnings`] so that function reads as one
/// sequence of mitigation filters rather than trailing off into wording.
fn sink_message(code: DiagCode, name: &str, sink_label: &str) -> String {
    match code {
        DiagCode::T100 => format!(
            "Tainted variable ${name} flows into {sink_label}; \
             possible code injection"
        ),
        DiagCode::T101 => format!(
            "Tainted variable ${name} flows into {sink_label}; \
             output may contain injected content"
        ),
        DiagCode::Irule3001 => format!(
            "Tainted variable ${name} in HTTP response body ({sink_label}); \
             risk of XSS or content injection"
        ),
        DiagCode::Irule3002 => format!(
            "Tainted variable ${name} in HTTP header/cookie value ({sink_label}); \
             risk of header injection"
        ),
        DiagCode::Irule3003 => format!(
            "Tainted variable ${name} in log output ({sink_label}); \
             risk of log injection or log forging"
        ),
        DiagCode::Irule3004 => format!(
            "Tainted variable ${name} in redirect URL ({sink_label}); \
             risk of open redirect"
        ),
        DiagCode::T104 => format!(
            "Tainted variable ${name} in network address argument of {sink_label}; \
             risk of SSRF (server-side request forgery)"
        ),
        DiagCode::T105 => format!(
            "Tainted variable ${name} in {sink_label} script argument; \
             risk of cross-interpreter code injection"
        ),
        _ => format!("Tainted variable ${name} flows into {sink_label}"),
    }
}

pub(in super::super) fn emit_sink_warnings<S: std::hash::BuildHasher>(
    env: &TaintScan<'_, S>,
    span: Span,
    code: DiagCode,
    sink_label: &str,
    call: &SinkCall<'_>,
    warnings: &mut Vec<TaintWarning>,
) {
    let mut names: std::collections::BTreeSet<&str> = env
        .uses
        .keys()
        .map(|&symbol| env.ssa.var_name(symbol))
        .collect();
    for index in 0..call.args.len() {
        if let Some((reference, _)) =
            original_sink_argument(call, index).and_then(WordExpr::sole_variable_substitution)
            && let Some(name) = reference_root_with_config(reference, call.source.config)
        {
            names.insert(name);
        }
    }
    for name in names {
        let original = (0..call.args.len())
            .filter_map(|index| original_sink_argument(call, index))
            .find(|word| {
                env.source_read.map_or_else(
                    || {
                        word.sole_variable_substitution()
                            .is_some_and(|(reference, _)| {
                                reference_root_with_config(reference, call.source.config)
                                    == Some(name)
                            })
                    },
                    |source| source.references(name, word, call.source.config),
                )
            });
        let Some(t) = operand_read_taint(env, name, original) else {
            continue;
        };
        if !t.is_tainted() {
            continue;
        }
        if env.ssa.var_symbol(name).is_some_and(|symbol| {
            env.quoted_uses
                .is_some_and(|quoted| quoted.contains(&symbol))
        }) && !quoted_sink_use_is_evaluated(call, name)
        {
            continue;
        }
        // The value reaching the sink is clean when every appearance of `name`
        // in the sink arguments is consumed by an embedded sanitiser
        // substitution — `puts [string length $x]` outputs the integer length,
        // not `$x` (tclsh-verified). The expr-operand path applies this via
        // word_taint; mirror it here for the direct sink-argument path.
        if var_consumed_by_sanitiser(call, name) {
            continue;
        }
        // What reaches the sink is the wrapper's result, not the variable, so
        // the mitigations below judge the wrapper's proofs rather than the
        // variable's — the same lattice the via-variable spelling would have
        // handed them.
        let t = var_wrapper_colours(call.registry, call.source, call.args, name, call.tokens, 0)
            .map_or(t, |colours| t.shape_unproven().with(colours));
        // Per-code mitigation suppression (T101, IRULE3001–3004).
        if sink_colour_mitigated(code, t) {
            continue;
        }
        // Registry-declared sink-safe colour (e.g. `exec` ← SHELL_ATOM): a
        // tainted value carrying the sink's safe colour cannot break out of its
        // dangerous slot — an IP/port atom can't word-split or inject shell
        // metacharacters into an `exec` argument. LIST_CANONICAL (eval/uplevel)
        // is excluded: its safety is position-dependent (the list *head* must be
        // a literal known command), handled by `list_wrapped_arg_command_is_literal`
        // below — a blanket check would wrongly clear `eval [list $raw]`, where
        // the tainted value is the command word.
        if let Some((_, command, _)) = selected_sink_schema(call)
            && let Some(safe) = tcl_registry::taint::taint_sink_safe_colour(call.registry, &command)
        {
            let safe = reg_colour(safe);
            if safe != TaintColour::LIST_CANONICAL && t.colours.contains(safe) {
                continue;
            }
        }
        if code == DiagCode::Irule3002
            && irule3002_name_position_safe(
                call.registry,
                call.command,
                call.args,
                name,
                t,
                original_sink_argument(call, 1),
                call.source.config,
            )
        {
            continue;
        }
        // Position-aware sink filter: a tainted variable only trips the
        // sink when it occupies a *dangerous* argument slot (the `puts`
        // content arg, a `taint_network_sink_args` network-address slot).
        if sink_var_position_safe(call, code, name) {
            continue;
        }
        // `eval`/`uplevel`/`interp eval [list <known-cmd> $v …]`: the
        // command word of the constructed list is a literal known command,
        // so the tainted `$v` is a quoted argument, not the command word —
        // no code-injection vector LIST_CANONICAL
        // head-literal filter).
        if matches!(code, DiagCode::T100 | DiagCode::T105)
            && list_wrapped_arg_command_is_literal(call, name)
        {
            continue;
        }
        // T104 admits a validated network-address colour. T105's List
        // quoting needs the separate literal-command-word proof above.
        if code == DiagCode::T104
            && t.colours
                .intersects(TaintColour::IP_ADDRESS | TaintColour::PORT | TaintColour::FQDN)
        {
            continue;
        }
        let message = sink_message(code, name, sink_label);
        warnings.push(TaintWarning {
            source_sink: None,
            span: sink_arg_span(call, name, span),
            variable: name.to_owned(),
            sink_command: sink_label.to_owned(),
            code,
            message,
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Whether a brace-quoted occurrence is in a registry-declared Tcl body slot.
/// A quoted word is literal data for `exec`/`puts`, but deferred source for
/// `eval`, `uplevel`, and `interp eval`; the role/trait projection owns that
/// distinction and the concatenating eval family extends through the tail.
///
/// The `None` read is deliberate under invariant I4: this is
/// a **widening** query, not hook selection — treating an
/// environment-disabled eval-family command as evaluating widens the taint
/// warning set (conservative), it never specialises semantics on an
/// unproved binding.
fn quoted_sink_use_is_evaluated(call: &SinkCall<'_>, name: &str) -> bool {
    let arg_refs: Vec<&str> = call.args.iter().map(String::as_str).collect();
    let traits = call
        .registry
        .invocation_traits(call.command, &arg_refs, None);
    let is_eval = traits.contains(Traits::EVALUATES_CODE)
        || classify_network_interp_sinks(call.registry, call.command, call.args)
            .iter()
            .any(|(code, _)| *code == DiagCode::T105);
    if !is_eval {
        return false;
    }
    let mut indices = call
        .registry
        .arg_indices_for_role(call.command, &arg_refs, ArgRole::Body);
    if traits.contains(Traits::SCRIPT_CONCATENATES_ARGS)
        && let Some(&first) = indices.first()
    {
        indices = (first..call.args.len()).collect();
    }
    indices.into_iter().any(|index| {
        call.args
            .get(index)
            .is_some_and(|arg| arg_var_names_with_config(arg, call.source.config).may_contain(name))
    })
}
