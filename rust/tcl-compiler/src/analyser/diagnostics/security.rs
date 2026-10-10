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

//! Injection and unsafe-evaluation security checks emitted during the
//! command walk.
//!
//! These diagnostics flag constructs that splice attacker-influenced data
//! into a command, an `eval`/`uplevel`/`interp eval` target, a `subst`
//! template, or a regular expression: string-concatenated `eval` (W101),
//! `subst` injection (W102), an open `exec`/`open` pipeline (W103), a
//! `catch` with no result variable that hides a failure (W302), a regex
//! prone to catastrophic backtracking (W303), a non-literal command word
//! where a literal is expected (W306), a `source` of a variable path
//! (W300), `uplevel` / `interp eval` injection (W301, W312),
//! double-decoded `eval [subst …]` (W309), closed value arguments left
//! open (W127), and a hardcoded credential literal (W310).

mod source_credentials;
mod source_crossing;
mod source_paths;
mod source_patterns;
mod source_reparse;
mod source_template;

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;
use tcl_registry::Traits;
use tcl_registry::arg_role::ArgRole;

impl Analyser {
    /// **W302.** Original selected error-capture syntax with one literal body
    /// and no result operand. A genuine single child invocation may suppress
    /// the hint through its independently selected teardown metadata. Recooked
    /// bodies cannot borrow source command geometry. Edits append only past a
    /// genuine whole written operand and retain the dialect's synopsis names.
    pub(in crate::analyser) fn emit_w302_catch_no_result_var(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if original.words().arguments().len() != 1
            || original
                .with_schema(|schema| schema.authored_source_descriptors().command.analyser_hook)
                .flatten()
                != Some(tcl_registry::hooks::AnalyserHookId::Catch)
            || !original
                .words()
                .roles()
                .is_some_and(|roles| roles.contains(&(0, ArgRole::Body)))
        {
            return;
        }
        let (Some(body), Some(literal)) = (original.word(0), original.literal(0)) else {
            return;
        };
        let Ok(content) = body.content_span() else {
            return;
        };
        // A recooked value cannot borrow the source body's command positions.
        if self.source.get(content.as_range()) != Some(literal) {
            return;
        }
        let children = crate::segmenter::segment_commands_with_offset_and_config(
            literal,
            content.start(),
            body.config(),
        );
        if let [child] = children.as_slice()
            && let Some(super::super::diagnostic_registry::OriginalDiagnosticSource::Registry(
                child,
            )) = self.original_diagnostic_source_for_segment(child)
            && child
                .with_schema(super::super::diagnostic_registry::source_descriptors)
                .is_some_and(|descriptors| {
                    descriptors
                        .command
                        .traits
                        .contains(Traits::FIRE_AND_FORGET_TEARDOWN)
                        || descriptors.subcommand.is_some_and(|sub| {
                            sub.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN)
                        })
                })
        {
            return;
        }
        let Some(subject) = original.subject(
            super::super::RegistrySourceDiagnosticKind::ErrorCapture,
            None,
        ) else {
            return;
        };
        let fixes = self.original_trailing_arg_fixes(original, "Add catch");
        self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
            DiagCode::W302, original.head().span(),
            "catch without a result variable silently swallows errors. Consider capturing the result: catch {\u{2026}} result".to_owned(),
            Severity::Hint).with_subject(subject).with_fixes(fixes));
    }

    /// Optional names and whole insertion extents from the same selected
    /// original invocation. Captured and expanded operands cannot issue edits.
    fn original_trailing_arg_fixes(
        &self,
        original: &super::super::diagnostic_registry::OriginalDiagnosticInvocation,
        title_prefix: &str,
    ) -> Vec<super::types::CodeFix> {
        let Some(documented) = original.with_schema(|schema| {
            schema.authored_source_optional_trailing_names(ArgRole::VarWrite)
        }) else {
            return Vec::new();
        };
        let Some(index) = original.words().arguments().len().checked_sub(1) else {
            return Vec::new();
        };
        if original.written_index(index).is_none() {
            return Vec::new();
        }
        let Some(last) = original.word(index) else {
            return Vec::new();
        };
        let insert_at = last.span().end();
        if insert_at as usize > self.source.len() {
            return Vec::new();
        }
        (1..=documented.len())
            .map(|count| {
                let names = documented[..count]
                    .iter()
                    .map(|placeholder| variable_name_for_placeholder(placeholder))
                    .collect::<Vec<_>>();
                super::types::CodeFix {
                    span: tcl_lexer::Span::new(insert_at, insert_at),
                    new_text: format!(" {}", names.join(" ")),
                    description: format!("{title_prefix} {} variable(s)", names.join(" + ")),
                    safety: super::types::FixSafety::BehaviourHardening,
                }
            })
            .collect()
    }

    /// W306: live original lexical substitutions in an independently selected
    /// regex pattern operand. An unresolved option boundary supplies no pattern
    /// role. Braced and pure-variable words remain literal/dynamic idioms.
    pub(in crate::analyser) fn emit_w306_literal_expected(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind as Kind, source_regex_pattern_arguments,
        };
        let Some(original) = original else { return };
        let Some(patterns) = original
            .with_schema(source_regex_pattern_arguments)
            .flatten()
        else {
            return;
        };
        for argument in patterns {
            let Some(word) = original.word(argument) else {
                continue;
            };
            if word.group().kind == tcl_lexer::WordKind::Braced {
                continue;
            }
            let arena = word.executable_parts();
            let parts = arena
                .list(arena.root())
                .iter()
                .filter(|part| !arena.text(part).is_some_and(<[u8]>::is_empty))
                .collect::<Vec<_>>();
            if matches!(parts.as_slice(), [part] if matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. }))
            {
                continue;
            }
            if !super::usage::original_word_has_substitution(word)
                || self.original_regex_quoting_substitution(word)
            {
                continue;
            }
            let Some(subject) = original.subject(Kind::PatternSubstitution, Some(argument)) else {
                continue;
            };
            let found = if arena
                .all_parts()
                .any(|part| matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. }))
            {
                "'$'"
            } else {
                "'['"
            };
            let advice = if word.group().kind == tcl_lexer::WordKind::Quoted {
                ". Use braces '{...}' instead of quotes."
            } else {
                ". Use braces '{...}' to prevent substitution."
            };
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W306,
                    word.span(),
                    format!(
                        "Literal expected in regular-expression pattern — found {found}{advice}"
                    ),
                    Severity::Warning,
                )
                .with_subject(subject),
            );
        }
    }

    /// The bracket component lends its original child body and full parser
    /// configuration. The retained analysis must select the child's authored
    /// metadata; a substring parse or same-spelled Registry lookup cannot do so.
    fn original_regex_quoting_substitution(&self, word: &tcl_lexer::NativeWord) -> bool {
        use crate::analyser::diagnostic_registry::{
            OriginalDiagnosticSource, source_declares_regex_quoting,
        };
        let arena = word.executable_parts();
        let mut parts = arena
            .list(arena.root())
            .iter()
            .filter(|part| !arena.text(part).is_some_and(<[u8]>::is_empty));
        let Some(part) = parts.next() else {
            return false;
        };
        if parts.next().is_some() {
            return false;
        }
        let tcl_lexer::ExecutablePart::Command { body } = part.part else {
            return false;
        };
        if word.image().bytes() != self.source.as_bytes()
            || self.result.body_lexer_config != Some(word.config())
        {
            return false;
        }
        let Some(source) = self.source.get(body.as_range()) else {
            return false;
        };
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            body.start(),
            word.config(),
        );
        let [command] = commands.as_slice() else {
            return false;
        };
        let Some(OriginalDiagnosticSource::Registry(selected)) =
            self.original_diagnostic_source_for_segment(command)
        else {
            return false;
        };
        selected.with_schema(source_declares_regex_quoting) == Some(true)
    }

    /// **W127.** A literal at a closed-value argument index is not in the
    /// command's allowed set.  Only fires
    /// for indices the spec marks `closed_value_args` (the `arg_values`
    /// are exhaustive, not hints). Unknown original values and declared
    /// option flags are skipped; literal substitution-like data stays data.
    pub(in crate::analyser) fn emit_w127_closed_value_args(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some((arguments, tokens, _)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        let cmd_name = original.command();
        let cmd_tok = *original
            .head()
            .tokens()
            .first()
            .expect("original head has a token");
        let Some(descriptors) =
            original.with_schema(super::super::diagnostic_registry::source_descriptors)
        else {
            return;
        };
        let mut hits: Vec<W127Hit> = Vec::new();
        {
            let spec = descriptors.command;
            let span_at = |i: usize| arg_tokens.get(i).map_or(cmd_tok.span, |t| t.span);
            // Top-level closed value args (exact match).
            let opt_names: std::collections::HashSet<&str> =
                spec.options.iter().map(|o| o.name).collect();
            for &idx in spec.closed_value_args {
                let i = idx as usize;
                let Some(value) = original.literal(i) else {
                    continue;
                };
                let values = spec.arg_values_at(idx);
                let allowed: Vec<&str> = values.iter().map(|av| av.value).collect();
                if let Some(hit) =
                    w127_closed_hit(value, span_at(i), &allowed, false, &opt_names, cmd_name)
                {
                    hits.push(hit);
                } else {
                    self.record_w137_gated_value(value, span_at(i), values, false, cmd_name);
                }
            }
            // Subcommand-level closed value args: the subcommand word occupies
            // arg 0, so the command-level index is one past the subcommand
            // relative index. `string is <class>` marks its class (`&[0]`),
            // matched by unique prefix (`arg_values_accept_prefix`).
            if let Some(sub) = descriptors.subcommand {
                let sub_opt_names: std::collections::HashSet<&str> =
                    sub.options.iter().map(|o| o.name).collect();
                let display = format!("{cmd_name} {}", args[0]);
                for &sub_idx in sub.closed_value_args {
                    let i = usize::from(sub_idx)
                        + original
                            .with_schema(|schema| schema.semantics.argument_offset)
                            .unwrap_or(0);
                    let Some(value) = original.literal(i) else {
                        continue;
                    };
                    let allowed: Vec<&str> = sub
                        .arg_values_at(sub_idx)
                        .iter()
                        .map(|av| av.value)
                        .collect();
                    if let Some(hit) = w127_closed_hit(
                        value,
                        span_at(i),
                        &allowed,
                        sub.arg_values_accept_prefix,
                        &sub_opt_names,
                        &display,
                    ) {
                        hits.push(hit);
                    } else {
                        self.record_w137_gated_value(
                            value,
                            span_at(i),
                            sub.arg_values_at(sub_idx),
                            sub.arg_values_accept_prefix,
                            &display,
                        );
                    }
                }
            }
        }
        for mut hit in hits {
            let Some(ordinal) = arg_tokens.iter().position(|token| token.span == hit.span) else {
                continue;
            };
            let Some(subject) = original.subject(
                super::super::diagnostic_registry::RegistrySourceDiagnosticKind::LiteralArgument,
                Some(ordinal),
            ) else {
                continue;
            };
            let Some(word) = original.word(ordinal) else {
                continue;
            };
            hit.span = word.span();
            if original.written_index(ordinal).is_none() {
                hit.fixes.clear();
            }
            for fix in &mut hit.fixes {
                fix.span = word.span();
            }
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W127,
                    hit.span,
                    hit.message,
                    Severity::Warning,
                )
                .with_fixes(hit.fixes)
                .with_subject(subject),
            );
        }
    }

    /// Buffer a W137 site when a **valid** closed argument value is
    /// version-gated ([`ArgValue::min_tcl`], the §6 argument-DSL rung):
    /// `string is dict` names a real class — on Tcl 9.0; below it the
    /// class itself raises at runtime. Decided post-walk against the
    /// effective Tcl version, like every version fact.
    ///
    /// [`ArgValue::min_tcl`]: tcl_registry::ArgValue
    fn record_w137_gated_value(
        &mut self,
        value: &str,
        span: tcl_lexer::Span,
        values: &'static [tcl_registry::ArgValue],
        accept_prefix: bool,
        display_name: &str,
    ) {
        let matched = values.iter().find(|av| av.value == value).or_else(|| {
            if !accept_prefix || value.is_empty() {
                return None;
            }
            // C Tcl's abbreviation rule: a unique prefix selects the value.
            let mut it = values.iter().filter(|av| av.value.starts_with(value));
            match (it.next(), it.next()) {
                (Some(av), None) => Some(av),
                _ => None,
            }
        });
        if let Some(av) = matched
            && let Some(min) = av.min_tcl
        {
            self.dsl_gate_sites.push(super::version_gate::DslGateSite {
                span,
                code: DiagCode::W137,
                what: format!("argument value '{}' of '{display_name}'", av.value),
                min,
            });
        }
    }

    /// **W127 (option-value sibling)** and **W141**. A literal value word of
    /// a value-taking option whose value set is *closed*
    /// (`OptionValue::enumerated(.., true, ..)`) is not in that set (W127) —
    /// unless the option also declares an [`tcl_registry::hover::IntegerDomain`]
    /// (`return -code ok|error|...|<int>`) and the value parses as a Tcl
    /// integer within it, which is accepted alongside the closed set rather
    /// than instead of it. Separately, an option whose arity is
    /// [`tcl_registry::hover::OptionArity::Hook`] runs its resolver as a
    /// content check: a `Some(msg)` `invalid` result is flagged as W141 (a
    /// value that's structurally malformed — `-errorstack`'s value must be
    /// an even-sized list — rather than merely outside a closed set).
    /// Options are matched by name or alias, arity and the `--` terminator
    /// are honoured by the shared source option scan. Both checks require
    /// original literal values or separately retained lattice values.
    pub(in crate::analyser) fn emit_w127_closed_option_values(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some((arguments, tokens, _)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        let cmd_name = original.command();
        let cmd_tok = *original
            .head()
            .tokens()
            .first()
            .expect("original head has a token");
        let Some(scan) = original
            .with_schema(super::super::diagnostic_registry::source_diagnostic_options)
            .flatten()
        else {
            return;
        };
        let mut hits: Vec<W127Hit> = Vec::new();
        let mut hook_hits: Vec<W127Hit> = Vec::new();
        for selected in scan
            .options
            .into_iter()
            .filter(|selected| selected.available)
        {
            let Some(values) = selected.values else {
                continue;
            };
            let Some(flag) = original.literal(selected.argument) else {
                continue;
            };
            let values = values.collect::<Vec<_>>();
            let occurrence = OptionOccurrence {
                original,
                cmd_name,
                arg: flag,
                opt: selected.option,
                vals: &values,
                args,
                arg_tokens,
                flag_idx: selected.argument,
                cmd_tok,
            };
            hits.extend(w127_domain_hits(&occurrence, self.grammar().numbers));
            hook_hits.extend(w141_hook_hit(&occurrence));
        }
        for mut hit in hits {
            let Some(ordinal) = arg_tokens.iter().position(|token| token.span == hit.span) else {
                continue;
            };
            let Some(subject) = original.subject(
                super::super::diagnostic_registry::RegistrySourceDiagnosticKind::LiteralArgument,
                Some(ordinal),
            ) else {
                continue;
            };
            let Some(word) = original.word(ordinal) else {
                continue;
            };
            hit.span = word.span();
            if original.written_index(ordinal).is_none() {
                hit.fixes.clear();
            }
            for fix in &mut hit.fixes {
                fix.span = word.span();
            }
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W127,
                    hit.span,
                    hit.message,
                    Severity::Warning,
                )
                .with_fixes(hit.fixes)
                .with_subject(subject),
            );
        }
        for mut hit in hook_hits {
            let Some(ordinal) = arg_tokens.iter().position(|token| token.span == hit.span) else {
                continue;
            };
            let Some(subject) = original.subject(
                super::super::diagnostic_registry::RegistrySourceDiagnosticKind::LiteralArgument,
                Some(ordinal),
            ) else {
                continue;
            };
            let Some(word) = original.word(ordinal) else {
                continue;
            };
            hit.span = word.span();
            if original.written_index(ordinal).is_none() {
                hit.fixes.clear();
            }
            for fix in &mut hit.fixes {
                fix.span = word.span();
            }
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W141,
                    hit.span,
                    hit.message,
                    Severity::Warning,
                )
                .with_fixes(hit.fixes)
                .with_subject(subject),
            );
        }
    }
}

/// True when `pattern` contains a catastrophic-backtracking shape: a
/// nested quantifier (`…+)+`, `…*)*`, `…+){`) or an overlapping
/// alternation (`(…|…)` immediately followed by `+` / `*` / `{`).
pub(super) fn has_redos_shape(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let quant = |b: Option<&u8>| matches!(b, Some(b'+' | b'*' | b'{'));
    // Nested quantifier: `<+|*> ) <+|*|{>`.
    for i in 0..bytes.len() {
        if matches!(bytes[i], b'+' | b'*')
            && bytes.get(i + 1) == Some(&b')')
            && quant(bytes.get(i + 2))
        {
            return true;
        }
    }
    // Overlapping alternation: `( [^)]* | [^)]* ) <+|*|{>`.
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut has_pipe = false;
            while j < bytes.len() && bytes[j] != b')' {
                has_pipe |= bytes[j] == b'|';
                j += 1;
            }
            if j < bytes.len() && has_pipe && quant(bytes.get(j + 1)) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Turn a synopsis placeholder into the variable name a quick-fix inserts.
///
/// Synopsis placeholders name a *role*, in Tcl manual-page style: the word
/// documented as `resultVarName` is the variable that receives the result.
/// Spliced into source verbatim it reads as documentation rather than code,
/// so the trailing `VarName` / `Name` marker is dropped and the leading
/// letter lower-cased — `resultVarName` becomes `result`,
/// `optionsVarName` becomes `options`, and a placeholder carrying no such
/// marker (`varName` alone, once `Name` is dropped, is `var`) keeps its
/// remaining spelling.  A placeholder that would reduce to nothing keeps
/// its original text so the fix always inserts a usable identifier.
fn variable_name_for_placeholder(placeholder: &str) -> String {
    let stem = placeholder
        .strip_suffix("VarName")
        .or_else(|| placeholder.strip_suffix("Name"))
        .filter(|stem| !stem.is_empty())
        .unwrap_or(placeholder);
    let mut chars = stem.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => placeholder.to_string(),
    }
}

/// One W127 closed-value check: return a `(message, span)` hit when the literal
/// value at command-level index `cmd_idx` is not among `allowed` — an exact
/// match, or (when `accept_prefix`) a unique prefix, mirroring C Tcl's
/// abbreviation rule for `string is <class>`. The caller supplies an original
/// literal or separately retained lattice value; variable-looking data stays
/// data. Declared option flags and an empty allowed set are skipped.
fn w127_closed_hit(
    value: &str,
    span: tcl_lexer::Span,
    allowed: &[&str],
    accept_prefix: bool,
    opt_names: &std::collections::HashSet<&str>,
    display_name: &str,
) -> Option<W127Hit> {
    if allowed.is_empty() || opt_names.contains(value) {
        return None;
    }
    let valid = if accept_prefix {
        // C Tcl's abbreviation rule (`Tcl_GetIndexFromObj`): an exact match
        // always wins; otherwise the value must be a *unique* prefix — an
        // abbreviation matching exactly one allowed value. An ambiguous prefix
        // (matching two or more, e.g. `a` → alnum/alpha/ascii) is a runtime
        // error, so it must NOT be treated as valid.
        allowed.contains(&value)
            || (!value.is_empty() && allowed.iter().filter(|a| a.starts_with(value)).count() == 1)
    } else {
        allowed.contains(&value)
    };
    if valid {
        return None;
    }
    let allowed_list = allowed.join(", ");
    let mut message =
        format!("Invalid value '{value}' for '{display_name}'; expected one of: {allowed_list}");
    let fixes = w127_suggestion_fix(value, allowed, span, &mut message);
    Some(W127Hit {
        message,
        span,
        fixes,
    })
}

/// One W127 finding: the message, its anchor, and the "did you mean…?"
/// replace fix (empty when no allowed value is close enough).
struct W127Hit {
    message: String,
    span: tcl_lexer::Span,
    fixes: Vec<crate::analyser::types::CodeFix>,
}

/// Append a "; did you mean 'X'?" suffix to `message` and build the
/// replace fix when one allowed value sits within the length-scaled edit
/// budget of `value`.  The allowed set is already in the message, so a
/// missing suggestion loses nothing.
/// One resolved option flag's context within the current command's args —
/// threaded through [`w127_domain_hits`]/[`w141_hook_hit`] instead of as
/// loose parameters, mirroring `commands::DispatchSite`'s bundled-context
/// shape.
struct OptionOccurrence<'a> {
    original: &'a super::super::diagnostic_registry::OriginalDiagnosticInvocation,
    cmd_name: &'a str,
    arg: &'a str,
    opt: &'a tcl_registry::hover::OptionSpec,
    vals: &'a [usize],
    args: &'a [String],
    arg_tokens: &'a [tcl_lexer::Token],
    flag_idx: usize,
    cmd_tok: tcl_lexer::Token,
}

/// Per-occurrence closed-set / integer-domain check for
/// [`Analyser::emit_w127_closed_option_values`] — one option's value
/// word(s) against its declared `values`/`integer`. Returns one hit per
/// invalid word; empty when the option declares neither (an open value).
fn w127_domain_hits(
    occ: &OptionOccurrence<'_>,
    numbers: tcl_dialect::NumberSyntax,
) -> Vec<W127Hit> {
    // Whether a literal is a Tcl integer is release-dependent (`08` and `1_0`
    // are integers from 9.0 and not before), so the domain check reads it under
    // the release being analysed rather than the ambient grammar.
    let allowed = occ.opt.value_values();
    let integer_domain = occ.opt.value_integer_domain();
    let has_closed_set = occ.opt.value_is_closed() && !allowed.is_empty();
    if !has_closed_set && integer_domain.is_none() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for &vi in occ.vals {
        let Some(value) = occ.original.literal(vi) else {
            continue;
        };
        let matches_literal = allowed.iter().any(|av| av.value == value);
        let matches_integer = integer_domain.is_some_and(|dom| {
            match tcl_syntax::number::parse_whole_with(
                value,
                tcl_syntax::number::ParseFlags::for_syntax(numbers),
            ) {
                Some(tcl_syntax::number::Number::Int(n)) => dom.accepts(n),
                // A bignum literal (too large for i64) is always outside a
                // bounded Range/Port, but still a legal Tcl integer for `Any`.
                Some(tcl_syntax::number::Number::Big { .. }) => {
                    matches!(dom, tcl_registry::hover::IntegerDomain::Any)
                }
                // A float/NaN literal is never a Tcl integer.
                Some(
                    tcl_syntax::number::Number::Double(_) | tcl_syntax::number::Number::Nan { .. },
                )
                | None => false,
            }
        });
        if matches_literal || matches_integer {
            continue;
        }
        let span = occ.arg_tokens.get(vi).map_or(occ.cmd_tok.span, |t| t.span);
        let (message, fixes) = if has_closed_set {
            let allowed_names: Vec<&str> = allowed.iter().map(|av| av.value).collect();
            let mut allowed_list = allowed_names.join(", ");
            if let Some(dom) = integer_domain {
                allowed_list.push_str(", or ");
                allowed_list.push_str(&describe_integer_domain(dom));
            }
            let mut message = format!(
                "Invalid value '{value}' for option '{}' on '{}'; expected one of: {allowed_list}",
                occ.arg, occ.cmd_name
            );
            let fixes = w127_suggestion_fix(value, &allowed_names, span, &mut message);
            (message, fixes)
        } else {
            // Pure numeric domain (`-level`): nothing in a literal set to
            // suggest against.
            let domain = integer_domain.expect("guarded by the outer `if`");
            (
                format!(
                    "Invalid value '{value}' for option '{}' on '{}'; expected {}",
                    occ.arg,
                    occ.cmd_name,
                    describe_integer_domain(domain)
                ),
                Vec::new(),
            )
        };
        hits.push(W127Hit {
            message,
            span,
            fixes,
        });
    }
    hits
}

/// Per-occurrence [`tcl_registry::hover::OptionArity::Hook`] content check
/// for [`Analyser::emit_w127_closed_option_values`] — `None` when the
/// option's arity isn't `Hook`, an original value is unknown, or the hook
/// reports the value valid.
fn w141_hook_hit(occ: &OptionOccurrence<'_>) -> Option<W127Hit> {
    let hook = occ.opt.value_arity_hook()?;
    if occ
        .vals
        .iter()
        .any(|&ordinal| occ.original.literal(ordinal).is_none())
    {
        return None;
    }
    let full: Vec<&str> = occ.args.iter().map(String::as_str).collect();
    let msg = hook(&full, occ.flag_idx + 1).invalid?;
    let span = occ
        .vals
        .first()
        .and_then(|&vi| occ.arg_tokens.get(vi))
        .map_or(occ.cmd_tok.span, |t| t.span);
    Some(W127Hit {
        message: format!("{msg} (option '{}' on '{}')", occ.arg, occ.cmd_name),
        span,
        fixes: Vec::new(),
    })
}

/// Human-readable description of an [`tcl_registry::hover::IntegerDomain`]
/// for a W127 message — `describe_integer_domain(Range(0, 2147483647))` →
/// `"an integer between 0 and 2147483647"`.
fn describe_integer_domain(domain: tcl_registry::hover::IntegerDomain) -> String {
    use tcl_registry::hover::IntegerDomain;
    match domain {
        IntegerDomain::Any => "an integer".to_string(),
        IntegerDomain::Range(lo, hi) => format!("an integer between {lo} and {hi}"),
        IntegerDomain::Port => "a port number (0-65535)".to_string(),
    }
}

fn w127_suggestion_fix(
    value: &str,
    allowed: &[&str],
    span: tcl_lexer::Span,
    message: &mut String,
) -> Vec<crate::analyser::types::CodeFix> {
    use std::fmt::Write as _;
    let suggestions = crate::text::suggest_similar(
        value,
        allowed.iter().copied(),
        1,
        crate::text::scaled_max_distance(value),
    );
    let Some(best) = suggestions.first() else {
        return Vec::new();
    };
    let _ = write!(message, "; did you mean '{best}'?");
    vec![crate::analyser::types::CodeFix {
        span,
        new_text: (*best).to_string(),
        description: format!("Replace with '{best}'"),
        // W127: an edit-distance guess at the intended value.
        safety: crate::irules_checks::FixSafety::RequiresReview,
    }]
}

/// True when `value` is a literal (not a `$var` / `[cmd]` substitution)
/// — the W310 literal-value gate.
fn is_literal_credential_value(value: &str, tok: &tcl_lexer::Token) -> bool {
    !matches!(
        tok.kind,
        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
    ) && !value.starts_with('$')
        && !value.contains('[')
}

/// The W102 finding for `cmd_name`'s template at `span`, naming the kinds
/// `performed` still runs and the switches that would narrow them.
fn w102_diagnostic(
    cmd_name: &str,
    span: tcl_lexer::Span,
    performed: tcl_registry::substitution::SubstitutionKinds,
    advice: Option<Vec<&'static str>>,
) -> crate::analyser::types::Diagnostic {
    let active = match (performed.commands, performed.variables) {
        (true, true) => "[cmd] and $var",
        (true, false) => "[cmd]",
        _ => "$var",
    };
    let advice = advice.map_or_else(
        || "Use [format] / [string map] for safe templating.".to_owned(),
        |switches| {
            format!(
                "Add {} to limit substitution scope, or use [format] / [string map] for safe \
templating.",
                switches.join(" ")
            )
        },
    );
    let message = format!(
        "{cmd_name} with a variable argument enables code injection: any {active} in the \
string will be evaluated. {advice}"
    );
    crate::analyser::types::Diagnostic::new(DiagCode::W102, span, message, Severity::Warning)
}

#[cfg(test)]
mod original_capture_edit_tests {
    use super::*;
    use crate::analyser::{DiagnosticSubject, ResolvedAnalysisInput};

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        input
    }

    #[test]
    fn original_captured_advice_keeps_readonly_subjects_without_call_operand_edits() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = input();
        for (written, captured, code, captured_text) in [
            (
                "catch {return VALUE}",
                "interp alias {} capture {} catch {return VALUE}; capture",
                DiagCode::W302,
                None,
            ),
            (
                "string is booleanx 1",
                "interp alias {} classify {} string is booleanx; classify 1",
                DiagCode::W127,
                Some("booleanx"),
            ),
            (
                "return -code errro VALUE",
                "interp alias {} fail {} return -code errro; fail VALUE",
                DiagCode::W127,
                Some("errro"),
            ),
        ] {
            let direct = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(written, "tcl");
            let direct = direct
                .diagnostics
                .iter()
                .find(|finding| finding.code == code)
                .expect(written);
            assert!(
                !direct.fixes.is_empty(),
                "genuine written operand retains existing proposal: {written}"
            );
            let result = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(captured, "tcl");
            let finding = result
                .diagnostics
                .iter()
                .find(|finding| finding.code == code)
                .expect(captured);
            assert!(
                finding.fixes.is_empty(),
                "capture owns no written operand here: {captured}"
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = finding.subject() else {
                panic!("original captured source subject")
            };
            assert!(subject.written_argument().is_none());
            if let Some(text) = captured_text {
                assert_eq!(&captured[finding.span.as_range()], text);
                assert!(finding.span.end() < u32::try_from(captured.rfind(';').unwrap()).unwrap());
            }
        }
    }

    #[test]
    fn original_captured_case_action_is_not_a_written_quoting_operand() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = input();
        let written = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse("switch -regexp subject literal BODY", "tcl");
        assert!(
            written
                .diagnostics
                .iter()
                .any(|finding| finding.code == DiagCode::W106)
        );
        let captured = Analyser::new().with_resolved_input(input).analyse(
            "interp alias {} choose {} switch -regexp subject literal BODY; choose",
            "tcl",
        );
        assert!(
            !captured
                .diagnostics
                .iter()
                .any(|finding| finding.code == DiagCode::W106)
        );
    }
}
