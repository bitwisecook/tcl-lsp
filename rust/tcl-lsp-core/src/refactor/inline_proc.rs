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
//! `a b` before and produces `{a b}` afterwards.  Nor is a
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
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
#[cfg(test)]
use tcl_dialect::BracedVarStyle;
use tcl_dialect::NumberSyntax;
use tcl_lexer::LexerConfig;
use tcl_registry::{ArgRole, CommandRegistry};

use super::{RefactorEdit, Refactoring};
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
/// not a refactor. With the oracle attached the head
/// simply does not resolve locally and no action is offered — the safe
/// answer, and the same one go-to-definition gives.
#[must_use]
pub fn inline_proc_in_program(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<Refactoring> {
    resolution.registry?;
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let registry = current.registry();
    let config = current.config();
    let resolution = resolution.with_registry(registry);
    let call = super::find_original_command_at(source, cursor, analysis)?;
    let head = call.name();
    if head.is_empty()
        || crate::definition::program_forced_import_abstains(
            analysis,
            resolution.program,
            call.span.start(),
        )
    {
        return None;
    }
    // Resolve the head exactly as the navigation providers do — the caller's
    // namespace candidates, the registry builtin gate, then the deterministic
    // simple-name fallback.  A namespace-blind `p.name == head` scan is the
    // drift class `cargo xtask resolution-drift` flags.
    let head_off = call.span.start();
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        head_off,
        &analysis.namespace_overrides,
    );
    let proc_def = if analysis.allows_retained_logical_declaration_advice() {
        tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
            source, analysis, head_off,
        )?
        .declaration(analysis)?
        .metadata()
    } else {
        crate::definition::resolve_called_proc(
            analysis, source, &namespace, head, head_off, resolution,
        )?
    };
    let title = format!("Inline proc '{}'", proc_def.name);
    let walk = super::FrameWalk::new(source, analysis)?;
    let (call_start, call_end) = walk.command_span_offsets(source, &call)?;

    match plan_inline(source, &call, proc_def, analysis, registry, config) {
        Ok(new_text) => {
            let proposal = Refactoring {
                title,
                edits: vec![RefactorEdit {
                    start: call_start,
                    end: call_end,
                    new_text,
                }],
                kind: ActionKind::RefactorInline,
                data_group: None,
                disabled: None,
            };
            proposal.check_replacement(source, |replacement| {
                check_inline_script(replacement, config)
            })?;
            Some(proposal)
        }
        Err(reason) => Some(Refactoring {
            title,
            edits: Vec::new(),
            kind: ActionKind::RefactorInline,
            data_group: None,
            disabled: Some(reason),
        }),
    }
}

fn check_inline_script(replacement: &str, config: LexerConfig) -> Option<()> {
    (super::source_rewrite::script_word_spellings(replacement, config.nested())?.len() == 1)
        .then_some(())
}

/// The inlined replacement text for `call`, or the reason it cannot be built.
fn plan_inline(
    source: &str,
    call: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
    _registry: &CommandRegistry,
    config: LexerConfig,
) -> Result<String, String> {
    // Literal substitution uses the independently retained lexical grammar.
    let profile = analysis
        .resolved_profile()
        .ok_or_else(|| "the original dialect is unavailable".to_owned())?;
    let numbers: NumberSyntax = profile.grammar.numbers;
    if !analysis.allows_retained_logical_declaration_advice() && proc_def.params_computed {
        return Err(
            "the proc's parameter list is computed at run time, so its formals are unknown"
                .to_string(),
        );
    }
    let body = single_command_body(source, proc_def, analysis, config)?;
    // Address every nested word in the full original document.
    let walk = super::FrameWalk::new(source, analysis)
        .ok_or_else(|| "the original document context is unavailable".to_owned())?;
    let mut nested = Vec::new();
    walk.nested_same_frame_commands(source, &body.command, &mut nested);
    if !walk.complete() {
        return Err(
            "a nested command's original frame or operand grammar is unavailable".to_owned(),
        );
    }
    if !analysis.allows_lexical_declaration_advice() {
        return original_inline_body(
            source,
            call,
            proc_def,
            analysis,
            &walk,
            InlineBodySyntax {
                command: &body,
                nested: &nested,
                numbers,
            },
        );
    }
    let bindings = bind_arguments(source, call, proc_def, &walk)?;
    reject_frame_sensitive_body(source, &body, &nested, &walk)?;
    reject_body_variable_writes(source, &body, &nested, &walk)?;
    let references = body_variable_references(source, &body, &nested, &walk)?;
    reject_args_reference(&references, proc_def, analysis)?;
    substitute_bindings(
        source,
        &body,
        &nested,
        &bindings,
        &walk,
        numbers,
        &references,
    )
}

#[derive(Clone, Copy)]
struct InlineBodySyntax<'a> {
    command: &'a BodyCommand,
    nested: &'a [tcl_compiler::segmenter::SegmentedCommand],
    numbers: NumberSyntax,
}

/// Rewrite only the selected original allocation and exact formal/argv
/// topology. Frozen values never erase a written variable or command read.
fn original_inline_body(
    source: &str,
    call: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
    walk: &super::FrameWalk<'_>,
    syntax: InlineBodySyntax<'_>,
) -> Result<String, String> {
    use tcl_compiler::registry_invocation::{effective_command_words, frozen_argument_words};
    let InlineBodySyntax {
        command: body,
        nested,
        numbers,
    } = syntax;
    let unavailable =
        || "the original parameter or call operand topology is unavailable".to_owned();
    let record = analysis
        .original_procedure_declarations()
        .find(|record| std::ptr::eq(record.metadata(), proc_def))
        .ok_or_else(unavailable)?;
    let registry = analysis.resolved_registry().ok_or_else(unavailable)?;
    let formals = analysis
        .original_procedure_formals(record, registry)
        .ok_or_else(unavailable)?;
    let config = analysis.body_lexer_config.ok_or_else(unavailable)?;
    let image = tcl_lexer::SourceImage::document(source);
    if !formals.matches_source(&image, config) {
        return Err(unavailable());
    }
    let tokens = walk.tokens(source, call);
    let binding = tokens.source_binding.as_ref().ok_or_else(unavailable)?;
    let effective = effective_command_words(&tokens).ok_or_else(unavailable)?;
    let frozen = frozen_argument_words(&tokens, &effective);
    let mut arguments = Vec::with_capacity(frozen.len());
    for (origin, value) in effective.origins.iter().skip(1).zip(&frozen) {
        let bytes = value
            .literal_bytes()
            .ok_or_else(|| "an argument's complete native value is unknown".to_owned())?;
        validate_inline_argument_origin(origin, bytes, &tokens, binding, &unavailable)?;
        arguments.push(bytes.to_vec());
    }
    if arguments.len() != effective.words.len().saturating_sub(1) {
        return Err(unavailable());
    }
    let rows = formals.bindings(arguments.len()).map_err(|error| {
        format!("the call's argument count does not bind its original formals: {error:?}")
    })?;
    let mut bindings = std::collections::BTreeMap::new();
    let mut rest_names = std::collections::BTreeSet::new();
    for row in rows {
        append_inline_formal_binding(
            row,
            &formals,
            &arguments,
            &mut bindings,
            &mut rest_names,
            &unavailable,
        )?;
    }
    let protocol = formals.original_input().policy().string_protocol();
    let mut references = std::collections::BTreeMap::new();
    let mut expression_ranges = Vec::new();
    for command in std::iter::once(&body.command).chain(nested) {
        collect_inline_body_references(
            source,
            command,
            walk,
            (&tokens, binding, registry),
            (&image, protocol, &rest_names, &bindings),
            (&mut references, &mut expression_ranges),
            &unavailable,
        )?;
    }
    let mut replacements = Vec::new();
    for ((start, end), name) in references {
        replacements.push(inline_bound_value_replacement(
            source,
            analysis,
            walk,
            InlineBodySyntax {
                command: body,
                nested,
                numbers,
            },
            (&image, protocol),
            (&bindings, &expression_ranges),
            (start, end, &name),
        )?);
    }
    let mut rewritten = body.text.clone();
    replacements.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    for (start, end, value) in replacements {
        rewritten.replace_range(start..end, &value);
    }
    Ok(rewritten)
}

/// One command's worth of proc body, re-segmented so its argument roles and
/// variable references can be inspected.
struct BodyCommand {
    /// The body text, braces stripped and trimmed.
    text: String,
    /// Exact original content origin in the full document.
    origin: u32,
    /// The body's single command, with absolute document spans.
    command: tcl_compiler::segmenter::SegmentedCommand,
}

/// Extract the proc's body and require it to be exactly one command.
fn single_command_body(
    source: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
    config: LexerConfig,
) -> Result<BodyCommand, String> {
    let image = tcl_lexer::SourceImage::document(source);
    let realm = analysis
        .retained_command_realm()
        .ok_or_else(|| "the proc's original body owner is unavailable".to_owned())?;
    let native_input = (!analysis.allows_lexical_declaration_advice())
        .then(|| {
            realm.original_written_name_input_at_span_in_source(&image, proc_def.body_span, config)
        })
        .flatten();
    let word = if analysis.allows_lexical_declaration_advice() {
        analysis
            .logical_source_procedure_declaration(proc_def)
            .and_then(|declaration| {
                declaration.original_body_word(&image, analysis.resolved_input.as_ref()?)
            })
    } else {
        native_input
            .as_ref()
            .and_then(|input| input.original_word_key())
            .map(tcl_compiler::signature_scan::original_name::SignatureSourceNameKey::original_word)
    }
    .filter(|word| {
        word.image() == &image
            && word.config() == config
            && word.group().kind == tcl_lexer::WordKind::Braced
            && !word.group().expand
            && word
                .tokens()
                .first()
                .is_some_and(|token| token.span == proc_def.body_span)
    })
    .ok_or_else(|| "the proc body has no current original literal word".to_owned())?;
    let span = word
        .content_span()
        .map_err(|_| "the body content extent is unavailable".to_owned())?;
    let raw = source
        .get(span.as_range())
        .ok_or_else(|| "the body content is unavailable".to_owned())?;
    let text = raw.trim();
    let origin = span.start()
        + u32::try_from(raw.len() - raw.trim_start().len())
            .map_err(|_| "the body is too large".to_owned())?;
    let commands = segment_commands_with_offset_and_config(
        text,
        origin,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    );
    let [command] = commands.as_slice() else {
        return Err(format!("the proc body is {} commands", commands.len()));
    };
    if command.is_partial {
        return Err("the proc body is incomplete".to_owned());
    }
    Ok(BodyCommand {
        text: text.to_owned(),
        origin,
        command: command.clone(),
    })
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

/// Bind only the actual current Logical source call and the agreed original
/// formal topology. These readonly operands feed the existing source recipe;
/// no activation, runtime equivalence or editing authority follows from them.
fn bind_arguments(
    source: &str,
    call: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &tcl_compiler::analyser::ProcDef,
    walk: &super::FrameWalk<'_>,
) -> Result<Vec<Binding>, String> {
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    let unavailable = || {
        "the call's original formal bindings are unavailable or would raise `wrong # args`"
            .to_owned()
    };
    let words = walk.native_words(source, call).ok_or_else(unavailable)?;
    if words.iter().any(|word| word.group().expand) {
        return Err("expanded arguments have no fixed original binding topology".to_owned());
    }
    let original =
        tcl_compiler::registry_invocation::source_structure::original_logical_procedure_arguments(
            source,
            walk.analysis,
            call,
        )
        .ok_or_else(unavailable)?;
    let image = tcl_lexer::SourceImage::document(source);
    let header = original.declaration();
    if !original.matches_source(&image, walk.config)
        || !original.matches_registry(walk.nesting)
        || original.original_words() != words
        || header.metadata().name_span != proc_def.name_span
        || header.metadata().body_span != proc_def.body_span
    {
        return Err(unavailable());
    }
    if original
        .arguments()
        .iter()
        .any(|value| value.literal_bytes().is_none())
    {
        return Err(
            "an argument is substituted at run time and cannot lose its evaluation".to_owned(),
        );
    }
    let parameters = header.original_source_formals().ok_or_else(unavailable)?;
    let mut bindings = Vec::new();
    for row in original.bindings() {
        let (parameter, value) = match *row {
            FormalByteArgumentBinding::Value {
                parameter,
                argument,
            } => {
                let value = original
                    .arguments()
                    .get(argument)
                    .and_then(|value| value.literal_bytes())
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .ok_or_else(|| {
                        "the argument's original static value is unavailable".to_owned()
                    })?;
                (parameter, value.to_owned())
            }
            FormalByteArgumentBinding::Default { parameter } => (
                parameter,
                parameters
                    .get(parameter)
                    .and_then(|parameter| parameter.default.clone())
                    .ok_or_else(unavailable)?,
            ),
            FormalByteArgumentBinding::Rest { .. } => continue,
            FormalByteArgumentBinding::CallerLink { .. } => {
                return Err(
                    "a caller-link formal selects a different frame after inlining".to_owned(),
                );
            }
        };
        bindings.push(Binding {
            name: parameters
                .get(parameter)
                .ok_or_else(unavailable)?
                .name
                .clone(),
            value,
            evaluation_is_observable: false,
        });
    }
    Ok(bindings)
}

/// Refuse a body whose command is frame-sensitive.
///
/// Membership comes from
/// the central selected-traits frame predicate — the registry's own union of
/// block terminators, control transfers, scope aliases, and barriers — so this
/// never names a command.  Moving any of them out of the proc's frame changes
/// what they return from, break out of, or bind against: `return` in an inlined
/// body would return from the *caller*, and `upvar 1 x y` would alias the
/// caller's caller instead of the caller.
fn reject_frame_sensitive_body(
    source: &str,
    body: &BodyCommand,
    nested: &[tcl_compiler::segmenter::SegmentedCommand],
    walk: &super::FrameWalk<'_>,
) -> Result<(), String> {
    for command in std::iter::once(&body.command).chain(nested) {
        let traits = walk
            .source_traits(source, command)
            .ok_or_else(|| "the body's original frame effects are unavailable".to_owned())?;
        if tcl_registry::traits::is_frame_sensitive(traits) {
            return Err(format!(
                "the body calls '{}', which acts on the call frame",
                command.name()
            ));
        }
    }
    Ok(())
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
    source: &str,
    body: &BodyCommand,
    nested: &[tcl_compiler::segmenter::SegmentedCommand],
    walk: &super::FrameWalk<'_>,
) -> Result<(), String> {
    for command in std::iter::once(&body.command).chain(nested) {
        let words = walk
            .source_words(source, command)
            .ok_or_else(|| "the body's original variable roles are unavailable".to_owned())?;
        let roles = words
            .roles()
            .ok_or_else(|| "the body's original variable roles are incomplete".to_owned())?;
        for &(ordinal, role) in roles {
            if !matches!(role, ArgRole::VarWrite | ArgRole::LoopVarList) {
                continue;
            }
            let word = words
                .arguments()
                .get(ordinal)
                .and_then(|word| word.literal_bytes())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .ok_or_else(|| "the body's written variable name is unavailable".to_owned())?;
            let names = if role == ArgRole::LoopVarList {
                tcl_syntax::word_rules::WordValueRules::from_grammar(
                    &walk.config.grammar_over(walk.dialect.grammar),
                )
                .split_list(word)
                .map_err(|_| "the body's variable list is malformed".to_owned())?
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect()
            } else {
                vec![word.to_owned()]
            };
            if !names.is_empty() {
                return Err(format!(
                    "the body assigns or binds '{}', which would leak into the caller's frame",
                    names.join("', '")
                ));
            }
        }
    }
    Ok(())
}

fn body_variable_references(
    source: &str,
    body: &BodyCommand,
    nested: &[tcl_compiler::segmenter::SegmentedCommand],
    walk: &super::FrameWalk<'_>,
) -> Result<Vec<super::VariableReference>, String> {
    let mut references = std::collections::BTreeMap::new();
    for command in std::iter::once(&body.command).chain(nested) {
        for reference in walk
            .variable_references(source, command)
            .ok_or_else(|| "the body's original variable components are unavailable".to_owned())?
        {
            if reference.array {
                return Err(
                    "an array reference needs its original activation and index evaluation"
                        .to_owned(),
                );
            }
            let key = (reference.span.start(), reference.span.end());
            if let Some(previous) = references.insert(key, reference.clone())
                && previous != reference
            {
                return Err("the body's original variable components disagree".to_owned());
            }
        }
    }
    if !walk.complete() {
        return Err("the body's original variable components are incomplete".to_owned());
    }
    Ok(references.into_values().collect())
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
    references: &[super::VariableReference],
    proc_def: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
) -> Result<(), String> {
    let source_parameters = analysis.logical_source_procedure_declaration(proc_def)
        .and_then(tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration::original_source_formals)
        .ok_or_else(|| "the original rest-formal source is unavailable".to_owned())?;
    if source_parameters
        .last()
        .is_none_or(|parameter| parameter.name != "args")
    {
        return Ok(());
    }
    if references.iter().any(|reference| reference.name == "args") {
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
    source: &str,
    body: &BodyCommand,
    nested: &[tcl_compiler::segmenter::SegmentedCommand],
    bindings: &[Binding],
    walk: &super::FrameWalk<'_>,
    numbers: NumberSyntax,
    references: &[super::VariableReference],
) -> Result<String, String> {
    // The occurrences come from the whole body text, so the expression
    // positions have to as well: `puts [expr {$n eq "abc"}]` puts its operand
    // one level below the body's own command, and substituting a bareword
    // there would make `expr` read it as a function name.
    let mut expr_ranges = Vec::new();
    for command in std::iter::once(&body.command).chain(nested) {
        for (start, end) in expr_argument_ranges(source, command, walk)? {
            expr_ranges.push((
                start
                    .checked_sub(body.origin as usize)
                    .ok_or("an expression lies outside the original body")?,
                end.checked_sub(body.origin as usize)
                    .ok_or("an expression lies outside the original body")?,
            ));
        }
    }
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for binding in bindings {
        let occurrences = references
            .iter()
            .filter(|reference| reference.name == binding.name)
            .collect::<Vec<_>>();
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
        if !tcl_syntax::word_rules::lexical_unicode_source_word_matches(
            &binding.value,
            &binding.value,
            walk.config.nested(),
        ) {
            return Err(format!(
                "'{}' is bound to `{}`, which is not a plain word — writing it \
                 into the body would change how the surrounding word is parsed",
                binding.name, binding.value
            ));
        }
        for reference in occurrences {
            let start = reference
                .span
                .start()
                .checked_sub(body.origin)
                .ok_or("a reference lies outside the original body")?
                as usize;
            let end = reference
                .span
                .end()
                .checked_sub(body.origin)
                .ok_or("a reference lies outside the original body")?
                as usize;
            if end > body.text.len() {
                return Err("a reference lies outside the original body".to_owned());
            }
            // An `expr` operand is a different language: a bare word there is
            // a function name or an error, not the string it spells.  The
            // positions come from the registry's `ArgRole::Expr`, so this
            // holds for every expression-taking command, not just `expr`.
            if expr_ranges
                .iter()
                .any(|(from, to)| start >= *from && end <= *to)
                && !is_expr_literal(&binding.value, numbers)
            {
                return Err(format!(
                    "'{}' is used as an expression operand and is bound to `{}`, \
                     which is not a number — `expr` reads a bare word as a \
                     function name, not as the string it spells",
                    binding.name, binding.value
                ));
            }
            let fragment = literal_substitution_container(
                source,
                walk.analysis,
                walk,
                body,
                nested,
                reference.span,
                &binding.value,
            )
            .ok_or_else(|| {
                "a plain word cannot preserve its actual source substitution container".to_owned()
            })?;
            replacements.push((start, end, fragment));
        }
    }
    let mut out = body.text.clone();
    replacements.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    for (start, end, value) in replacements {
        out.replace_range(start..end, &value);
    }
    Ok(out)
}

/// Literal construction under the actual receiver's selected Tcl/Expr source
/// grammar. A successful correspondence grants no binding or relocation fact.
fn literal_substitution_container(
    source: &str,
    analysis: &AnalysisResult,
    walk: &super::FrameWalk<'_>,
    body: &BodyCommand,
    nested: &[tcl_compiler::segmenter::SegmentedCommand],
    reference: tcl_lexer::Span,
    value: &str,
) -> Option<String> {
    let mut admitted = Vec::new();
    for command in std::iter::once(&body.command).chain(nested) {
        let words = walk.native_words(source, command)?;
        let schema = walk.source_words(source, command)?;
        schema.roles()?;
        let roles = schema.written_argument_roles();
        for (ordinal, word) in words.iter().enumerate() {
            if word.span().start() > reference.start() || reference.end() > word.span().end() {
                continue;
            }
            let expression = ordinal
                .checked_sub(1)
                .is_some_and(|argument| roles.contains(&(argument, ArgRole::Expr)));
            let matches = if expression {
                let profile = analysis.resolved_profile()?;
                // Quoted Tcl arguments may transform expression source before
                // expr sees it. No decoded value is invented for that language.
                if word.group().kind == tcl_lexer::WordKind::Quoted {
                    continue;
                }
                let Ok(expression_span) = word.content_span() else {
                    continue;
                };
                let Some(text) = word
                    .image()
                    .try_text()
                    .ok()
                    .and_then(|text| text.get(expression_span.as_range()))
                else {
                    continue;
                };
                let Some(from) = reference.start().checked_sub(expression_span.start()) else {
                    continue;
                };
                let Some(to) = reference.end().checked_sub(expression_span.start()) else {
                    continue;
                };
                if to > expression_span.end() - expression_span.start() {
                    continue;
                }
                let mut context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
                context.lexer_grammar = word.config().grammar_over(profile.grammar);
                tcl_syntax::expr::OriginalSourceExpression::capture(text, &context, word.config())
                    .and_then(|expression| {
                        expression.scalar_numeric_literal_substitution(
                            tcl_lexer::Span::new(from, to),
                            value,
                        )
                    })
            } else {
                tcl_syntax::word_rules::lexical_source_scalar_literal_substitution(
                    word, reference, value,
                )
                .then(|| value.to_owned())
            };
            if let Some(fragment) = matches {
                admitted.push(fragment);
            }
        }
    }
    (admitted.len() == 1).then(|| admitted.pop().unwrap())
}

/// The byte ranges of `command`'s expression-role arguments, within the body
/// text the command was segmented from.
///
/// Read off the registry's [`ArgRole::Expr`], so `expr`, `if`, `while`, and
/// `for`'s condition are all covered without this module naming any of them.
fn expr_argument_ranges(
    source: &str,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    walk: &super::FrameWalk<'_>,
) -> Result<Vec<(usize, usize)>, String> {
    let words = walk
        .source_words(source, command)
        .ok_or_else(|| "the body's original expression roles are unavailable".to_owned())?;
    words
        .roles()
        .ok_or_else(|| "the body's original expression roles are incomplete".to_owned())?;
    words
        .written_argument_roles()
        .into_iter()
        .filter(|(_, role)| *role == ArgRole::Expr)
        .map(|(ordinal, _)| {
            command
                .argv
                .get(ordinal + 1)
                .map(|token| (token.span.start() as usize, token.span.end() as usize))
                .ok_or_else(|| "the original expression word is unavailable".to_owned())
        })
        .collect()
}

#[cfg(test)]
fn variable_references(text: &str, style: BracedVarStyle) -> Vec<(String, usize, usize)> {
    super::syntax_variable_references(
        text,
        LexerConfig {
            braced_var: style,
            ..LexerConfig::default()
        },
    )
    .unwrap_or_default()
    .into_iter()
    .map(|reference| {
        (
            reference.name,
            reference.span.start() as usize,
            reference.span.end() as usize,
        )
    })
    .collect()
}

/// `true` when `value` can be written into any Tcl word position verbatim
/// and still denote itself.
///
/// The permitted set is the characters that carry no meaning to the parser in
/// any context: no whitespace (which would split the word), and none of
/// `$ [ ] { } " \ ; #` (substitution, grouping, quoting, command separation,
/// and comment introduction).  An empty value is excluded too — it needs
/// quoting to survive as a word at all.
#[cfg(test)]
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

fn validate_inline_argument_origin(
    origin: &tcl_compiler::registry_invocation::InvocationWordOrigin,
    bytes: &[u8],
    tokens: &tcl_compiler::ir::CommandTokens,
    binding: &tcl_compiler::command_binding::SourceInvocationBinding,
    unavailable: &impl Fn() -> String,
) -> Result<(), String> {
    use tcl_compiler::registry_invocation::InvocationWordOrigin;
    match origin {
        InvocationWordOrigin::Written(written) => {
            let input = binding
                .original_written_name_input(tokens, *written)
                .ok_or_else(unavailable)?;
            let key = input.original_word_key().ok_or_else(|| {
                "inlining would remove an observable written argument evaluation".to_owned()
            })?;
            if key.bytes() != bytes {
                return Err(unavailable());
            }
        }
        InvocationWordOrigin::ExpandedElement { written, element } => {
            let parent = binding
                .original_written_name_input(tokens, *written)
                .ok_or_else(unavailable)?;
            if parent.original_static_list_container().is_none() {
                return Err("inlining would remove a substituted list evaluation".to_owned());
            }
            let child = parent
                .original_list_element(*element)
                .ok_or_else(unavailable)?;
            if child.bytes() != bytes {
                return Err(unavailable());
            }
        }
        InvocationWordOrigin::BindingPrefix(_) => {}
        InvocationWordOrigin::ResolvedHead => return Err(unavailable()),
    }
    Ok(())
}

type InlineBodyValues<'image, 'names, 'bindings> = (
    &'image tcl_lexer::SourceImage,
    tcl_syntax::native_string::NativeStringProtocol,
    &'names std::collections::BTreeSet<Vec<u8>>,
    &'bindings std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
);

type InlineBodyReferences<'references, 'expressions> = (
    &'references mut std::collections::BTreeMap<(u32, u32), Vec<u8>>,
    &'expressions mut Vec<tcl_lexer::Span>,
);

fn collect_inline_body_references(
    source: &str,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    walk: &super::FrameWalk<'_>,
    call: (
        &tcl_compiler::ir::CommandTokens,
        &tcl_compiler::command_binding::SourceInvocationBinding,
        &tcl_registry::CommandRegistry,
    ),
    values: InlineBodyValues<'_, '_, '_>,
    output: InlineBodyReferences<'_, '_>,
    unavailable: &impl Fn() -> String,
) -> Result<(), String> {
    let (tokens, binding, registry) = call;
    let (image, protocol, rest_names, bindings) = values;
    let (references, expression_ranges) = output;
    let selected = walk
        .structure(source, command)
        .ok_or_else(|| "the original body command structure is unavailable".to_owned())?;
    let body_tokens = walk.tokens(source, command);
    let head = body_tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.original_written_name_input(&body_tokens, 0))
        .and_then(|input| input.original_word_key().cloned())
        .ok_or_else(|| "the copied command head has no original static producer".to_owned())?;
    // This narrow proposal facade addresses authored Registry names. A
    // body alias or qualified/opaque head needs its separate byte proposal
    // lookup; re-encoding a displayed or resolved name would lose identity.
    if !selected.facts.canonical_command.is_ascii()
        || head.bytes() != selected.facts.canonical_command.as_bytes()
        || !binding.original_proposed_registry_commands(
            tokens,
            registry,
            &[&selected.facts.canonical_command],
        )
    {
        return Err(
            "the copied command's implementation is not preserved in the caller namespace"
                .to_owned(),
        );
    }
    if tcl_registry::traits::is_frame_sensitive(selected.facts.traits) {
        return Err("the body calls a command that acts on the call frame".to_owned());
    }
    if selected
        .facts
        .arg_roles
        .iter()
        .any(|(_, role)| matches!(role, ArgRole::VarWrite | ArgRole::LoopVarList))
    {
        return Err("the body assigns or binds variables in its own frame".to_owned());
    }
    if selected.facts.body_kind != tcl_registry::BodyKind::Plain
        && selected
            .facts
            .arg_roles
            .iter()
            .any(|(_, role)| matches!(role, ArgRole::Body | ArgRole::LambdaLiteral))
    {
        return Err("the body opens a distinct naming or variable frame".to_owned());
    }
    for (written, role) in selected.written_argument_roles() {
        if role == ArgRole::Expr {
            let token = command.argv.get(written + 1).ok_or_else(unavailable)?;
            expression_ranges.push(token.span);
        }
        if matches!(role, ArgRole::VarRead) {
            return Err("the body reads a variable through a name operand".to_owned());
        }
    }
    for arena in walk.components(source, command).ok_or_else(unavailable)? {
        for part in arena.all_parts() {
            let tcl_lexer::ExecutablePart::Variable { name, index } = part.part else {
                continue;
            };
            let name = tcl_syntax::backslash::native_source_literal_bytes(
                arena.bytes(name).ok_or_else(unavailable)?,
                image.channel(),
                protocol,
            )
            .map_err(|_| unavailable())?
            .into_owned();
            if index.is_some() {
                return Err(
                    "an array reference needs its original activation and index evaluation"
                        .to_owned(),
                );
            }
            if rest_names.contains(&name) {
                return Err(
                    "the body reads a variadic list whose word quoting cannot be preserved"
                        .to_owned(),
                );
            }
            if !bindings.contains_key(&name) {
                return Err(
                    "the body reads a variable outside its original formal bindings".to_owned(),
                );
            }
            references.insert((part.span.start(), part.span.end()), name);
        }
    }
    Ok(())
}

fn inline_bound_value_replacement(
    source: &str,
    analysis: &AnalysisResult,
    walk: &super::FrameWalk<'_>,
    syntax: InlineBodySyntax<'_>,
    owner: (
        &tcl_lexer::SourceImage,
        tcl_syntax::native_string::NativeStringProtocol,
    ),
    values: (
        &std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
        &[tcl_lexer::Span],
    ),
    reference: (u32, u32, &[u8]),
) -> Result<(usize, usize, String), String> {
    let InlineBodySyntax {
        command: body,
        nested,
        numbers,
    } = syntax;
    let (image, protocol) = owner;
    let (bindings, expression_ranges) = values;
    let (start, end, name) = reference;
    let unavailable =
        || "the original parameter or call operand topology is unavailable".to_owned();
    let bytes = bindings.get(name).ok_or_else(unavailable)?;
    let text = tcl_syntax::backslash::native_literal_source_text(bytes, image.channel(), protocol)
        .filter(|text| {
            tcl_syntax::word_rules::lexical_unicode_source_word_matches(
                text,
                text,
                walk.config.nested(),
            )
        })
        .ok_or_else(|| {
            "a bound native value cannot be written as an inert literal in the body".to_owned()
        })?;
    if expression_ranges
        .iter()
        .any(|span| start >= span.start() && end <= span.end())
        && !is_expr_literal(&text, numbers)
    {
        return Err("a nonnumeric bound value cannot replace an expression variable".to_owned());
    }
    let text = literal_substitution_container(
        source,
        analysis,
        walk,
        body,
        nested,
        tcl_lexer::Span::new(start, end),
        &text,
    )
    .ok_or_else(|| {
        "a bound value cannot preserve its actual source substitution container".to_owned()
    })?;
    let start = start.checked_sub(body.origin).ok_or_else(unavailable)? as usize;
    let end = end.checked_sub(body.origin).ok_or_else(unavailable)? as usize;
    body.text.get(start..end).ok_or_else(unavailable)?;
    Ok((start, end, text))
}

fn append_inline_formal_binding(
    row: tcl_syntax::formal_params::FormalByteArgumentBinding,
    formals: &tcl_compiler::signature_scan::formal_parameters::SignatureSourceFormalParameters,
    arguments: &[Vec<u8>],
    bindings: &mut std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
    rest_names: &mut std::collections::BTreeSet<Vec<u8>>,
    unavailable: &impl Fn() -> String,
) -> Result<(), String> {
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    match row {
        FormalByteArgumentBinding::Value {
            parameter,
            argument,
        } => {
            bindings.insert(
                formals
                    .parameters()
                    .get(parameter)
                    .ok_or_else(unavailable)?
                    .name
                    .clone(),
                arguments.get(argument).ok_or_else(unavailable)?.clone(),
            );
        }
        FormalByteArgumentBinding::Default { parameter } => {
            let parameter = formals
                .parameters()
                .get(parameter)
                .ok_or_else(unavailable)?;
            bindings.insert(
                parameter.name.clone(),
                parameter.default.clone().ok_or_else(unavailable)?,
            );
        }
        FormalByteArgumentBinding::Rest { name, .. } => {
            rest_names.insert(name);
        }
        FormalByteArgumentBinding::CallerLink { .. } => {
            return Err(
                "a caller-link formal would select a different frame after inlining".to_owned(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_inline_tests;

#[cfg(test)]
mod original_inline_destination_tests;

#[cfg(test)]
mod selected_source_body_tests;

#[cfg(test)]
mod original_inline_body_purpose_tests;

#[cfg(test)]
mod source_binding_transport_tests;

#[cfg(test)]
mod retained_literal_container_tests;
