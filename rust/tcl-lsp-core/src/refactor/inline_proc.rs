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

//! Inline proc — replace a call with the proc's body, with its parameters
//! bound to the call's arguments.
//!
//! # What Tcl actually does, and why a textual splice is not it
//!
//! `f {a b}` does not pass the four characters `{a b}` to `f`; it passes the
//! three-character *value* `a b`, because the braces are the caller's quoting
//! and are consumed by the parse.  Splicing the written word into the body
//! therefore changes the value the body sees — the body's `$name` produced
//! `a b` before and produces `{a b}` afterwards (issue #1199).  Nor is a
//! parameter with no written argument simply absent: `proc f {{name world}}`
//! called as `f` binds `name` to `world`, so an inlining that drops the
//! parameter leaves the body reading an unset variable.
//!
//! Both are consequences of the same thing: inlining is a *binding* problem,
//! not a text problem.  This transform therefore
//!
//! * performs real parameter binding — required parameters, defaults, and
//!   the variadic `args` tail, with the same arity rules C Tcl applies;
//! * computes each argument's **value**, not its spelling; and
//! * refuses, with a plain-English reason, wherever the value cannot be
//!   written back into the body's context without changing what it means.
//!
//! # What it refuses, and why
//!
//! Refusal is not failure — it is the correct answer whenever the rewrite
//! would not preserve behaviour.  The reasons are surfaced on the code action
//! (LSP's `disabled.reason`), so the editor greys the entry out and explains
//! itself rather than silently offering nothing.
//!
//! | Refused | Because |
//! |---|---|
//! | The head does not resolve to a proc in this file | There is no body to inline. |
//! | The body is not exactly one command | Splicing several commands into one caller word changes the parse; a multi-command body needs statement-level insertion this transform does not do. |
//! | The body uses a frame-sensitive command (`upvar`, `uplevel`, `global`, `variable`, `return`, `break`, `continue`, `info level`, …) | Those read or write the *call frame*. Moving them into the caller's frame changes which variables they bind, what they return from, and what they break out of. Membership comes from the registry's own frame-sensitivity union, never a keyword list here. |
//! | The body writes a variable | A proc's locals vanish when it returns; a caller's do not. Inlining a `set` leaks a variable into the caller and can clobber one of the same name. |
//! | The call supplies too few or too many arguments | Tcl would raise `wrong # args`; inlining would silently not. |
//! | The body reads `args` | `args` holds a *list* of the trailing values. Writing that list back into an arbitrary word context re-quotes it, and there is no spelling that is correct in every context. |
//! | An argument's value is not a plain word | Only a value made of characters that are inert everywhere (no whitespace, quotes, braces, brackets, `$`, `\`, `;`, or `#`) can be written into the body verbatim and still mean the same thing. |
//! | A parameter used inside an `expr` operand is bound to a non-numeric value | `expr {$n * 2}` with `n` bound to `abc` is a variable read; `expr {abc * 2}` is an invalid bareword. The `expr` positions come from the registry's `ArgRole::Expr`, not from knowing what `expr` is. |
//! | A parameter used more than once is bound to a side-effecting argument | `f [next]` with the body reading `$x` twice would call `next` twice instead of once. |
//!
//! # Worked examples
//!
//! ```tcl
//! proc double {x} { expr {$x * 2} }
//! double 5                  ;# inlines to  expr {5 * 2}
//!
//! proc greet {{name world}} { puts $name }
//! greet                     ;# inlines to  puts world   (the default binds)
//!
//! proc greet {name} { puts "hello $name" }
//! greet {a b}               ;# refused: the value `a b` is not a plain word
//! ```

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::segment_commands_with_offset;
use tcl_dialect::{BracedVarStyle, NumberSyntax};
use tcl_lexer::{Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use super::{RefactorEdit, Refactoring, command_span_offsets, find_command_at, token_end_offset};
use crate::code_actions::ActionKind;

/// Inline the proc called at byte offset `cursor`.
///
/// Returns `None` when the cursor is not on a call to a resolvable proc at
/// all — there is nothing to offer.  Returns a [`Refactoring`] whose
/// `disabled` reason is set when a call *is* found but cannot be inlined
/// without changing behaviour: the action is still surfaced, greyed out, so
/// the user learns why rather than wondering where it went.
#[must_use]
pub fn inline_proc(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Option<Refactoring> {
    inline_proc_in_program(
        source,
        cursor,
        analysis,
        crate::definition::CallResolution::document_only().with_registry(registry),
    )
}

/// [`inline_proc`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Inlining substitutes *the body of the proc the call actually reaches*, so
/// a `namespace import -force` whose covering `namespace export` lives in
/// another file makes inlining the local same-named proc a behaviour change,
/// not a refactor (issue #1116 item 1). With the oracle attached the head
/// simply does not resolve locally and no action is offered — the safe
/// answer, and the same one go-to-definition gives.
#[must_use]
pub fn inline_proc_in_program(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<Refactoring> {
    let registry = resolution.registry?;
    let call = find_command_at(source, cursor, None, registry)?;
    let head = call.name();
    if head.is_empty() {
        return None;
    }
    // Resolve the head exactly as the navigation providers do — the caller's
    // namespace candidates, the registry builtin gate, then the deterministic
    // simple-name fallback.  A namespace-blind `p.name == head` scan is the
    // M1 drift class `cargo xtask resolution-drift` flags.
    let head_off = call.span.start();
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        head_off,
        &analysis.namespace_overrides,
    );
    let proc_def = crate::definition::resolve_called_proc(
        analysis, source, &namespace, head, head_off, resolution,
    )?;
    let title = format!("Inline proc '{}'", proc_def.name);
    let (call_start, call_end) = command_span_offsets(source, &call);

    // The numeric-literal grammar of the dialect this document was analysed
    // under — whether a substituted value reads as a number in an `expr`
    // operand is release-dependent (`0o17` from 8.5, `0d99` and `1_000` from
    // 9.0).  `AnalysisResult::dialect` carries the name the host passed to
    // `Analyser::analyse`; an empty (default-constructed) one resolves to the
    // permissive `plain_tcl` profile, i.e. modern rules.
    let numbers = crate::profile_for_dialect(&analysis.dialect)
        .grammar
        .numbers;
    // …and its `${…}` close rule, for the same reason: which bytes are the
    // variable's name is release-dependent, and this transform rewrites the
    // reference's own span (issue #1605).
    let style = super::braced_var_style(analysis);

    match plan_inline(source, &call, proc_def, registry, numbers, style) {
        Ok(new_text) => Some(Refactoring {
            title,
            edits: vec![RefactorEdit {
                start: call_start,
                end: call_end,
                new_text,
            }],
            kind: ActionKind::RefactorInline,
            data_group: None,
            disabled: None,
        }),
        Err(reason) => Some(Refactoring {
            title,
            edits: Vec::new(),
            kind: ActionKind::RefactorInline,
            data_group: None,
            disabled: Some(reason),
        }),
    }
}

/// The inlined replacement text for `call`, or the reason it cannot be built.
fn plan_inline(
    source: &str,
    call: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &tcl_compiler::analyser::ProcDef,
    registry: &CommandRegistry,
    numbers: NumberSyntax,
    style: BracedVarStyle,
) -> Result<String, String> {
    if proc_def.params_computed {
        return Err(
            "the proc's parameter list is computed at run time, so its formals are unknown"
                .to_string(),
        );
    }
    let body = single_command_body(source, proc_def)?;
    let bindings = bind_arguments(source, call, proc_def)?;
    reject_frame_sensitive_body(&body, registry)?;
    reject_body_variable_writes(&body, registry)?;
    reject_args_reference(&body, proc_def, style)?;
    substitute_bindings(&body, &bindings, registry, numbers, style)
}

/// One command's worth of proc body, re-segmented so its argument roles and
/// variable references can be inspected.
struct BodyCommand {
    /// The body text, braces stripped and trimmed.
    text: String,
    /// The body's single command, with spans relative to `text`.
    command: tcl_compiler::segmenter::SegmentedCommand,
}

/// Extract the proc's body and require it to be exactly one command.
fn single_command_body(
    source: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> Result<BodyCommand, String> {
    let span = proc_def.body_span;
    let raw = source
        .get(span.start() as usize..span.end() as usize)
        .unwrap_or("")
        .trim();
    // `body_span` may exclude the proc's closing `}` (the lexer's inner-end
    // convention), so strip a trailing `}` only when it is the unbalanced
    // *outer* brace — a greedy `trim_end_matches('}')` would eat an inner
    // sub-expression brace (`expr {$n * 2}`) and leave unparseable text.
    let inner = raw.strip_prefix('{').map_or(raw, str::trim_start);
    let text = if inner.matches('}').count() > inner.matches('{').count() {
        let trimmed = inner.trim_end();
        trimmed.strip_suffix('}').unwrap_or(trimmed).trim()
    } else {
        inner.trim()
    }
    .to_string();
    if text.is_empty() {
        return Err("the proc body is empty".to_string());
    }
    let commands: Vec<_> = segment_commands_with_offset(&text, 0)
        .into_iter()
        .filter(|command| !command.name().is_empty())
        .collect();
    if commands.len() != 1 {
        return Err(format!(
            "the proc body is {} commands; inlining several commands into a \
             single caller word would change how the caller parses",
            commands.len()
        ));
    }
    let command = commands.into_iter().next().expect("length checked above");
    Ok(BodyCommand { text, command })
}

/// A parameter bound to the value the call gives it.
struct Binding {
    /// Parameter name as declared.
    name: String,
    /// The value the parameter receives, as a Tcl *value* — not the caller's
    /// spelling of it.
    value: String,
    /// Whether producing that value runs code (a `[…]` substitution) or reads
    /// a variable, so re-evaluating it more than once is observable.
    evaluation_is_observable: bool,
}

/// Bind the call's arguments to the proc's formals, applying C Tcl's rules:
/// positional parameters in order, declared defaults for the ones the call
/// omits, and the variadic `args` tail collecting whatever is left.
///
/// Errors on an argument count C Tcl would reject (`wrong # args`): inlining
/// must not turn a call that raises into one that quietly works.
fn bind_arguments(
    source: &str,
    call: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> Result<Vec<Binding>, String> {
    let params = &proc_def.params;
    let has_args_tail = params.last().is_some_and(|param| param.name == "args");
    let fixed = if has_args_tail {
        params.len() - 1
    } else {
        params.len()
    };
    let required = params
        .iter()
        .take(fixed)
        .filter(|param| !param.has_default)
        .count();
    let supplied = call.argv.len().saturating_sub(1);
    if supplied < required {
        return Err(format!(
            "the call passes {supplied} argument(s) but '{}' requires {required} — \
             C Tcl would raise `wrong # args`",
            proc_def.name
        ));
    }
    if !has_args_tail && supplied > fixed {
        return Err(format!(
            "the call passes {supplied} argument(s) but '{}' accepts at most {fixed} — \
             C Tcl would raise `wrong # args`",
            proc_def.name
        ));
    }
    let mut bindings = Vec::new();
    for (index, param) in params.iter().take(fixed).enumerate() {
        // No written argument means the declared default binds.  Only a
        // parameter that *has* one can reach here — the required count was
        // checked above — so an absent default is an unreachable empty value.
        let (value, evaluation_is_observable) = match call.argv.get(index + 1) {
            Some(&token) => argument_value(source, token)?,
            None => (param.default_value.clone().unwrap_or_default(), false),
        };
        bindings.push(Binding {
            name: param.name.clone(),
            value,
            evaluation_is_observable,
        });
    }
    Ok(bindings)
}

/// The **value** a call argument denotes, plus whether producing it is
/// observable (runs a command or reads a variable).
///
/// This is the step a textual splice skips.  A braced word `{a b}` denotes
/// the value `a b`; a quoted word `"a b"` denotes `a b`; a bare word denotes
/// itself.  A word carrying a live substitution has no statically-known
/// value at all, and is reported as such so the caller can refuse.
fn argument_value(source: &str, token: Token) -> Result<(String, bool), String> {
    let start = token.span.start() as usize;
    let end = token_end_offset(source, token) as usize;
    let raw = source
        .get(start..end)
        .ok_or_else(|| "the call argument's source range is unreadable".to_string())?;
    match token.kind {
        // A braced word is a literal: its contents reach the command
        // byte-for-byte, with no substitution applied.
        TokenType::Str => Ok((
            raw.strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
                .unwrap_or(raw)
                .to_string(),
            false,
        )),
        TokenType::Var | TokenType::Cmd => Err(format!(
            "the argument `{raw}` is substituted at run time, so its value is \
             not known here"
        )),
        _ => {
            if raw.starts_with('"') {
                let inner = raw
                    .strip_prefix('"')
                    .and_then(|rest| rest.strip_suffix('"'))
                    .unwrap_or(raw);
                if inner.contains('$') || inner.contains('[') || inner.contains('\\') {
                    return Err(format!(
                        "the argument `{raw}` is substituted at run time, so its \
                         value is not known here"
                    ));
                }
                return Ok((inner.to_string(), false));
            }
            if raw.contains('$') || raw.contains('[') || raw.contains('\\') {
                return Err(format!(
                    "the argument `{raw}` is substituted at run time, so its value \
                     is not known here"
                ));
            }
            Ok((raw.to_string(), false))
        }
    }
}

/// Refuse a body whose command is frame-sensitive.
///
/// Membership comes from
/// [`CommandRegistry::frame_sensitive_commands`] — the registry's own union of
/// block terminators, control transfers, scope aliases, and barriers — so this
/// never names a command.  Moving any of them out of the proc's frame changes
/// what they return from, break out of, or bind against: `return` in an inlined
/// body would return from the *caller*, and `upvar 1 x y` would alias the
/// caller's caller instead of the caller.
fn reject_frame_sensitive_body(
    body: &BodyCommand,
    registry: &CommandRegistry,
) -> Result<(), String> {
    let unsafe_heads = registry.frame_sensitive_commands();
    let head = body.command.name();
    if unsafe_heads.contains(&head) {
        return Err(format!(
            "the body calls '{head}', which acts on the call frame — running it \
             in the caller's frame would change what it returns from, breaks out \
             of, or binds against"
        ));
    }
    // A frame-sensitive command nested in a `[…]` substitution inside an
    // argument is just as frame-bound as one at the head.
    for text in body.command.args() {
        if let Some(inner) = nested_command_head(text)
            && unsafe_heads.contains(&inner)
        {
            return Err(format!(
                "the body evaluates '{inner}', which acts on the call frame — \
                 running it in the caller's frame would change what it binds \
                 against"
            ));
        }
    }
    Ok(())
}

/// The head word of a `[…]` command substitution embedded anywhere in `text`.
fn nested_command_head(text: &str) -> Option<&str> {
    let open = text.find('[')?;
    let rest = text.get(open + 1..)?;
    let head = rest
        .split(|c: char| c.is_ascii_whitespace() || c == ']')
        .next()?;
    (!head.is_empty()).then_some(head)
}

/// Refuse a body that writes a variable.
///
/// A proc's locals disappear when it returns; the caller's do not.  Inlining
/// `set total 0` therefore leaves `total` behind in the caller — and clobbers
/// the caller's own `total` if it had one.  The written positions come from
/// the registry's [`ArgRole::VarWrite`] role, so this covers `set`, `incr`,
/// `lappend`, `append`, `dict set`, `lassign`, `regexp -inline`'s capture
/// variables, and anything else a spec declares, without listing any of them.
fn reject_body_variable_writes(
    body: &BodyCommand,
    registry: &CommandRegistry,
) -> Result<(), String> {
    let args: Vec<&str> = body.command.args().iter().map(String::as_str).collect();
    let writes = registry.arg_indices_for_role(body.command.name(), &args, ArgRole::VarWrite);
    if writes.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the body assigns a variable ('{}' writes to '{}'), which would leak \
         into — and could overwrite — a variable of the same name in the caller",
        body.command.name(),
        writes
            .iter()
            .filter_map(|index| args.get(*index))
            .copied()
            .collect::<Vec<_>>()
            .join("', '")
    ))
}

/// Refuse a body that reads the variadic `args` list.
///
/// `args` holds a *list* of the trailing argument values.  Writing that list
/// back into the body's word context re-quotes it — braced in one place, split
/// into several words in another — and there is no single spelling that is
/// correct in every context, so this transform does not guess.  A proc that
/// *declares* `args` but never reads it is fine: the binding simply has no
/// occurrence to substitute.
fn reject_args_reference(
    body: &BodyCommand,
    proc_def: &tcl_compiler::analyser::ProcDef,
    style: BracedVarStyle,
) -> Result<(), String> {
    if proc_def.params.last().is_none_or(|p| p.name != "args") {
        return Ok(());
    }
    if variable_references(&body.text, style)
        .into_iter()
        .any(|(name, _, _)| name == "args")
    {
        return Err(
            "the body reads `args`, whose value is a list — there is no spelling \
             of a list that means the same thing in every word context"
                .to_string(),
        );
    }
    Ok(())
}

/// Substitute each binding's value for its `$param` / `${param}` references,
/// refusing wherever the value cannot be written into that position without
/// changing what it means.
fn substitute_bindings(
    body: &BodyCommand,
    bindings: &[Binding],
    registry: &CommandRegistry,
    numbers: NumberSyntax,
    style: BracedVarStyle,
) -> Result<String, String> {
    let references = variable_references(&body.text, style);
    let expr_ranges = expr_argument_ranges(&body.command, registry);
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for binding in bindings {
        let occurrences: Vec<&(String, usize, usize)> = references
            .iter()
            .filter(|(name, _, _)| *name == binding.name)
            .collect();
        if occurrences.is_empty() {
            continue;
        }
        if occurrences.len() > 1 && binding.evaluation_is_observable {
            return Err(format!(
                "'{}' is used {} times in the body but its argument is evaluated \
                 at run time — inlining would evaluate it {} times instead of once",
                binding.name,
                occurrences.len(),
                occurrences.len()
            ));
        }
        if !is_plain_word(&binding.value) {
            return Err(format!(
                "'{}' is bound to `{}`, which is not a plain word — writing it \
                 into the body would change how the surrounding word is parsed",
                binding.name, binding.value
            ));
        }
        for (_, start, end) in occurrences {
            // An `expr` operand is a different language: a bare word there is
            // a function name or an error, not the string it spells.  The
            // positions come from the registry's `ArgRole::Expr`, so this
            // holds for every expression-taking command, not just `expr`.
            if expr_ranges
                .iter()
                .any(|(from, to)| *start >= *from && *end <= *to)
                && !is_expr_literal(&binding.value, numbers)
            {
                return Err(format!(
                    "'{}' is used as an expression operand and is bound to `{}`, \
                     which is not a number — `expr` reads a bare word as a \
                     function name, not as the string it spells",
                    binding.name, binding.value
                ));
            }
            replacements.push((*start, *end, binding.value.clone()));
        }
    }
    let mut out = body.text.clone();
    replacements.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    for (start, end, value) in replacements {
        out.replace_range(start..end, &value);
    }
    Ok(out)
}

/// The byte ranges of `command`'s expression-role arguments, within the body
/// text the command was segmented from.
///
/// Read off the registry's [`ArgRole::Expr`], so `expr`, `if`, `while`, and
/// `for`'s condition are all covered without this module naming any of them.
fn expr_argument_ranges(
    command: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> Vec<(usize, usize)> {
    let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
    registry
        .arg_indices_for_role(command.name(), &args, ArgRole::Expr)
        .into_iter()
        .filter_map(|index| {
            // `arg_indices_for_role` is 0-based over the post-name arguments;
            // the representative argv token sits one further along.
            let token = command.argv.get(index + 1)?;
            Some((token.span.start() as usize, token.span.end() as usize))
        })
        .collect()
}

/// `(name, start, end)` for every `$name` / `${name}` reference in `text`,
/// where `start..end` covers the whole reference including the `$` and any
/// braces.
///
/// Matching whole references rather than doing a `str::replace` is what keeps
/// `$nn` intact when the parameter is `n`, and stops a `$name` embedded in a
/// longer token being half-rewritten.
/// Every `$name` / `${name}` reference in `text`, with the byte span of the
/// whole reference — the span this transform rewrites, so it must be the
/// span the document's own release would parse (issue #1605).
fn variable_references(text: &str, style: BracedVarStyle) -> Vec<(String, usize, usize)> {
    super::variable_reference_spans(text, style)
}

/// `true` when `value` can be written into any Tcl word position verbatim
/// and still denote itself.
///
/// The permitted set is the characters that carry no meaning to the parser in
/// any context: no whitespace (which would split the word), and none of
/// `$ [ ] { } " \ ; #` (substitution, grouping, quoting, command separation,
/// and comment introduction).  An empty value is excluded too — it needs
/// quoting to survive as a word at all.
fn is_plain_word(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|c| {
            !c.is_whitespace()
                && !matches!(
                    c,
                    '$' | '[' | ']' | '{' | '}' | '"' | '\\' | ';' | '#' | '(' | ')'
                )
        })
}

/// `true` when the whole of `value` is a numeric literal `expr` reads as
/// itself, under `numbers` — the numeric-literal grammar of the dialect the
/// document was analysed under.
///
/// `expr` treats a bare non-numeric word as a function name (or an error), so
/// only a value `expr` parses as a number can replace a variable reference in
/// an expression operand.  The test is therefore exactly the one C's
/// `ParseLexeme` makes before classifying an `expr` lexeme as a `NUMBER`, and
/// runs through the one shared numeral facility: a private recogniser here
/// accepted any alphanumeric run after a radix prefix, so `0xZZZ`, `0b999`, and
/// `0o8` all passed as numbers and the refactor spliced them into an `expr`
/// operand — turning a working script into a runtime error.
fn is_expr_literal(value: &str, numbers: NumberSyntax) -> bool {
    tcl_syntax::number::is_whole_number(value, numbers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    /// Run the transform at the first occurrence of `needle` in `src`, with the
    /// document analysed under `dialect`.
    fn at_dialect(
        src: &str,
        needle: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Option<Refactoring> {
        let registry = super::super::test_registry();
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, dialect.name).clone();
        let cursor = u32::try_from(src.find(needle).expect("needle in source")).unwrap() + 1;
        inline_proc(src, cursor, &analysis, &registry)
    }

    /// Run the transform at the first occurrence of `needle` in `src`.
    fn at(src: &str, needle: &str) -> Option<Refactoring> {
        at_dialect(
            src,
            needle,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        )
    }

    /// The rewritten document, or the refusal reason, under `dialect`.
    fn outcome_for(
        src: &str,
        needle: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Result<String, String> {
        let refactoring = at_dialect(src, needle, dialect).expect("a call to inline");
        match &refactoring.disabled {
            Some(reason) => Err(reason.clone()),
            None => Ok(refactoring.apply(src)),
        }
    }

    /// The rewritten document, or the refusal reason.
    fn outcome(src: &str, needle: &str) -> Result<String, String> {
        outcome_for(
            src,
            needle,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        )
    }

    // -- TP: binding is performed and the result is correct ---------------

    #[test]
    fn tp_inlines_a_literal_argument() {
        let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 5\n";
        assert_eq!(
            outcome(src, "double 5").unwrap(),
            "proc double {x} {\n    expr {$x * 2}\n}\nexpr {5 * 2}\n"
        );
    }

    #[test]
    fn tp_binds_a_declared_default_when_the_call_omits_the_argument() {
        // C Tcl 9.0.3 prints `hello world` for the original.  The old textual
        // splice emitted `puts $name`, which errors on an unset variable.
        let src = "proc greet {{name world}} {\n    puts $name\n}\ngreet\n";
        assert_eq!(
            outcome(src, "greet\n").unwrap(),
            "proc greet {{name world}} {\n    puts $name\n}\nputs world\n"
        );
    }

    #[test]
    fn tp_a_written_argument_overrides_the_default() {
        let src = "proc greet {{name world}} {\n    puts $name\n}\ngreet there\n";
        assert_eq!(
            outcome(src, "greet there").unwrap(),
            "proc greet {{name world}} {\n    puts $name\n}\nputs there\n"
        );
    }

    #[test]
    fn tp_repeated_parameter_use_is_fine_for_a_literal() {
        let src = "proc sq {n} {\n    expr {$n * $n}\n}\nsq 7\n";
        assert_eq!(
            outcome(src, "sq 7").unwrap(),
            "proc sq {n} {\n    expr {$n * $n}\n}\nexpr {7 * 7}\n"
        );
    }

    #[test]
    fn tp_an_unread_args_tail_does_not_block_inlining() {
        let src = "proc shout {word args} {\n    puts $word\n}\nshout hi extra\n";
        assert_eq!(
            outcome(src, "shout hi").unwrap(),
            "proc shout {word args} {\n    puts $word\n}\nputs hi\n"
        );
    }

    #[test]
    fn tp_a_prefix_sharing_parameter_name_is_not_corrupted() {
        // `$nn` must survive when the parameter is `n`.
        let src = "proc f {n} {\n    puts $n$nn\n}\nf 1\n";
        let result = outcome(src, "f 1\n").unwrap();
        assert!(result.ends_with("puts 1$nn\n"), "{result}");
    }

    // -- FP: refusals that keep behaviour ---------------------------------

    #[test]
    fn fp_refuses_a_braced_argument_whose_value_is_not_a_plain_word() {
        // The issue's second reproducer.  Original prints `hello a b`; the
        // textual splice emitted `puts "hello {a b}"`, printing the braces.
        let src = "proc greet {name} {\n    puts \"hello $name\"\n}\ngreet {a b}\n";
        let reason = outcome(src, "greet {a b}").unwrap_err();
        assert!(reason.contains("plain word"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_substituted_argument() {
        let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble $n\n";
        let reason = outcome(src, "double $n").unwrap_err();
        assert!(reason.contains("substituted at run time"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_non_numeric_value_in_an_expression_operand() {
        // `expr {abc * 2}` reads `abc` as a function name, not the string.
        let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble abc\n";
        let reason = outcome(src, "double abc").unwrap_err();
        assert!(reason.contains("not a number"), "{reason}");
    }

    #[test]
    fn fp_refuses_too_few_arguments() {
        let src = "proc pair {a b} {\n    puts $a$b\n}\npair 1\n";
        let reason = outcome(src, "pair 1\n").unwrap_err();
        assert!(reason.contains("wrong # args"), "{reason}");
    }

    #[test]
    fn fp_refuses_too_many_arguments() {
        let src = "proc one {a} {\n    puts $a\n}\none 1 2\n";
        let reason = outcome(src, "one 1 2").unwrap_err();
        assert!(reason.contains("wrong # args"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_body_that_reads_args() {
        let src = "proc all {args} {\n    puts $args\n}\nall 1 2\n";
        let reason = outcome(src, "all 1 2").unwrap_err();
        assert!(reason.contains("list"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_body_that_assigns_a_variable() {
        // `total` would survive in the caller and clobber a same-named one.
        let src = "proc reset {} {\n    set total 0\n}\nreset\n";
        let reason = outcome(src, "reset\n").unwrap_err();
        assert!(reason.contains("assigns a variable"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_body_using_upvar() {
        let src = "proc bind {name} {\n    upvar 1 $name local\n}\nbind x\n";
        let reason = outcome(src, "bind x\n").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_body_using_return() {
        let src = "proc answer {} {\n    return 42\n}\nanswer\n";
        let reason = outcome(src, "answer\n").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_body_using_global() {
        let src = "proc grab {} {\n    global config\n}\ngrab\n";
        let reason = outcome(src, "grab\n").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_multi_command_body() {
        let src = "proc two {} {\n    puts a\n    puts b\n}\ntwo\n";
        let reason = outcome(src, "two\n").unwrap_err();
        assert!(reason.contains("commands"), "{reason}");
    }

    #[test]
    fn fp_refuses_an_empty_body() {
        let src = "proc nothing {} {}\nnothing\n";
        let reason = outcome(src, "nothing\n").unwrap_err();
        assert!(reason.contains("empty"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_computed_parameter_list() {
        let src = "proc makeargs {} {return {a b}}\nproc p [makeargs] {\n    puts $a\n}\np 1 2\n";
        let reason = outcome(src, "p 1 2").unwrap_err();
        assert!(reason.contains("computed"), "{reason}");
    }

    // -- TN: nothing to offer ---------------------------------------------

    #[test]
    fn tn_no_action_on_a_builtin_call() {
        assert!(at("puts hello\n", "puts").is_none());
    }

    #[test]
    fn tn_no_action_on_an_unresolvable_head() {
        assert!(at("frobnicate 1\n", "frobnicate").is_none());
    }

    #[test]
    fn tn_no_action_outside_any_command() {
        assert!(at("\n\n", "\n").is_none());
    }

    // -- Unit-level predicates --------------------------------------------

    #[test]
    fn plain_word_rejects_every_parser_significant_character() {
        assert!(is_plain_word("abc"));
        assert!(is_plain_word("1.5"));
        assert!(is_plain_word("a-b_c.d/e"));
        assert!(!is_plain_word(""));
        for value in ["a b", "a$b", "a[b", "a{b", "a\"b", "a\\b", "a;b", "a#b"] {
            assert!(!is_plain_word(value), "{value} must not be a plain word");
        }
    }

    #[test]
    fn expr_literal_accepts_what_expr_parses_as_a_number() {
        let n = NumberSyntax::Tcl90;
        for value in ["0", "42", "-7", "+7", "1.5", "1e3", "0xff", "0b1010"] {
            assert!(is_expr_literal(value, n), "{value} is a number");
        }
        for value in ["abc", "", "-", "1a2"] {
            assert!(!is_expr_literal(value, n), "{value} is not a number");
        }
    }

    /// The bug the shared numeral facility fixes: the old recogniser accepted
    /// any *alphanumeric* run after a radix prefix, so `0xZZZ` / `0b999` /
    /// `0o8` passed as numbers.  Splicing one of those into an `expr` operand
    /// turns a working script into a runtime error — `expr` reads a
    /// non-numeric bareword as a function name.
    #[test]
    fn expr_literal_rejects_radix_invalid_digits() {
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            for value in ["0xZZZ", "0b999", "0o8", "0b2", "0x", "0x_", "--5", "+-5"] {
                assert!(
                    !is_expr_literal(value, syntax),
                    "{value} is not a number under {syntax:?}"
                );
            }
        }
    }

    /// A radix prefix only exists from the release that added it: `0o`/`0b`
    /// from 8.5, `0d` from 9.0, `_` digit separators from 9.0.  Before that the
    /// word is a bareword, which `expr` would read as a function name.
    #[test]
    fn expr_literal_gates_radix_prefixes_on_the_release() {
        for syntax in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
            assert!(is_expr_literal("0o17", syntax), "{syntax:?}");
        }
        assert!(!is_expr_literal("0o17", NumberSyntax::Tcl84));
        assert!(is_expr_literal("0d99", NumberSyntax::Tcl90));
        assert!(is_expr_literal("1_000", NumberSyntax::Tcl90));
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert!(!is_expr_literal("0d99", syntax), "{syntax:?}");
            assert!(!is_expr_literal("1_000", syntax), "{syntax:?}");
        }
    }

    /// End-to-end: the dialect the document was analysed under decides whether
    /// an `0o17` argument may be substituted into an `expr` operand.
    #[test]
    fn fp_refuses_a_radix_prefix_the_target_release_lacks() {
        let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 0o17\n";
        assert_eq!(
            outcome_for(
                src,
                "double 0o17",
                tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile()
            )
            .unwrap(),
            "proc double {x} {\n    expr {$x * 2}\n}\nexpr {0o17 * 2}\n"
        );
        let reason = outcome_for(
            src,
            "double 0o17",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        )
        .unwrap_err();
        assert!(reason.contains("not a number"), "{reason}");
    }

    /// The bug end-to-end: `0xZZZ` is not a number in any release, so the
    /// refactor must refuse rather than emit `expr {0xZZZ * 2}`.
    #[test]
    fn fp_refuses_radix_invalid_digits_in_an_expr_operand() {
        let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 0xZZZ\n";
        let reason = outcome(src, "double 0xZZZ").unwrap_err();
        assert!(reason.contains("not a number"), "{reason}");
    }

    #[test]
    fn variable_references_finds_whole_names_only() {
        let found = variable_references("puts $n$nn ${n}x", BracedVarStyle::Tcl9Nesting);
        let names: Vec<&str> = found.iter().map(|(name, _, _)| name.as_str()).collect();
        assert_eq!(names, vec!["n", "nn", "n"]);
    }

    /// Issue #1605 — inline-proc **rewrites** each reference's own byte
    /// span, so the span must be the one the document's release parses. On a
    /// 9.x document `${a{b}c}` is one reference spanning all 8 bytes; on 8.x
    /// it ends at the first `}` and the trailing `c}` is word text that must
    /// survive the substitution untouched.
    ///
    /// Oracle: `puts ${a{b}c}` prints `NINE` on tclsh 9.0.4 and `EIGHTc}` on
    /// 8.6.16.
    #[test]
    fn variable_reference_spans_follow_the_documents_release() {
        let text = "puts ${a{b}c}";
        let nine = variable_references(text, BracedVarStyle::Tcl9Nesting);
        assert_eq!(nine.len(), 1, "{nine:?}");
        assert_eq!(nine[0].0, "a{b}c");
        // `$` at 5 through the closing `}` at 12 inclusive.
        assert_eq!((nine[0].1, nine[0].2), (5, 13));

        let eight = variable_references(text, BracedVarStyle::FirstClose);
        assert_eq!(eight.len(), 1, "{eight:?}");
        assert_eq!(eight[0].0, "a{b");
        // The reference stops before `c}`, which stays ordinary word text.
        assert_eq!((eight[0].1, eight[0].2), (5, 11));
        assert_eq!(&text[eight[0].2..], "c}");
    }
}
