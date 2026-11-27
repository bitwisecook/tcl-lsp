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

//! Reads in the words of one command: structural bodies, nested substitutions, expression roles and braced words that own a name.

use super::super::{SsaInvocationContext, UseClass};
use super::ClassifiedUses;
use crate::ir::CommandTokens;
use crate::var_refs::VarReferenceScanner;
use std::collections::{BTreeSet, HashSet};
use tcl_registry::CommandRegistry;
use tcl_source::words::WordExpr;

// Variable-use extraction
//
// These functions determine which variables an IR statement reads.

/// Return `true` when argument at `arg_index` is a braced literal
/// (single-token STR word).
///
/// When token info is unavailable, returns `false` so unknown
/// arguments are still scanned as ordinary inputs. We only exclude
/// bodies when we can positively identify them as single-token
/// braced literals.
fn is_braced_arg(tokens: Option<&CommandTokens>, arg_index: usize) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    // tokens.argv includes the command name at index 0; args are 1-based.
    let tok_index = arg_index + 1;
    if tok_index >= tokens.single_token_word.len() {
        return false;
    }
    if !tokens.single_token_word[tok_index] {
        return false;
    }
    // A single-token word from a VAR or CMD token is not braced.
    if let Some(text) = tokens.argv_texts.get(tok_index) {
        !text.starts_with("${") && !text.starts_with('[')
    } else {
        true
    }
}

/// Return BODY arg indices that should be excluded from local statement uses.
///
/// We only exclude handler-style bodies that are lowered/analysed separately.
/// Dynamic evaluation commands like `eval` still need their args treated as
/// ordinary dataflow inputs (for taint and read-before-set tracking).
pub(crate) fn structural_body_indices(
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> HashSet<usize> {
    use tcl_registry::{ArgRole, BodyKind};

    // A foreign-dialect builtin disabled in the active dialect — known in some
    // dialect but absent from the active registry's `by_name` (the analyser
    // loads only the active dialect, so an iRules `when` / `log` / `session`
    // misses under plain Tcl) — is an unknown would-be user command here. Tcl
    // never substitutes inside its braced arguments, so every braced arg is
    // opaque *data*, not analysable script/expr; scanning it would read its
    // `$vars` and emit spurious findings (W210). Skip them. A command
    // unknown in *every* dialect (`get` miss *and* not known-in-any) — a real
    // user proc / TclOO body / recovery artefact — is NOT skipped: its braced
    // body still recurses.
    if registry.get(command).is_none() && registry.known_in_any_dialect(command) {
        return (0..args.len())
            .filter(|&idx| is_braced_arg(tokens, idx))
            .collect();
    }

    // The registry-declared `body_kind` on each spec /
    // subcommand tells us whether body args run in the caller's
    // frame (`Plain`) or in a separate definition / dispatch context
    // (`Structural`).  Only `Structural` body args belong in this
    // skip set — `if`, `while`, `for`, `foreach`, `catch`, `try`,
    // … bodies share the caller's frame and SSA must scan them as
    // part of the enclosing block's data flow.
    // An `ArgRole::LambdaLiteral` word is a whole anonymous procedure —
    // `{params body ?ns?}` — and C Tcl runs it in a **fresh frame**, so no
    // name written inside it is a read (or write) in the frame the call is
    // written in.  tclsh 9.0.4 / 8.6.14, identical:
    //
    //   set x 7; apply {{} {puts $x}}   ;# can't read "x": no such variable
    //
    // The lambda's own body is analysed on its own, in that frame, with its
    // parameter list bound (`lowering::lower_apply` registers it as a body
    // unit) — so scanning the literal here both mis-frames the read and
    // duplicates it, drawing a false `W210` on the lambda's own parameters
    // Unconditional, not gated on `body_kind`: the role is
    // itself the "fresh frame" statement.  A *dynamic* lambda (`apply
    // $lambda`) is not braced, so `is_braced_arg` keeps it a genuine read.
    let lambda_words: HashSet<usize> = {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        registry
            .arg_indices_for_role(command, &arg_strs, ArgRole::LambdaLiteral)
            .into_iter()
            .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
            .collect()
    };

    if let Some(spec) = registry.get(command) {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Subcommand body_kind (if the call dispatches to a sub).
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind == BodyKind::Structural {
            let candidates = registry.arg_indices_for_role(command, &arg_strs, ArgRole::Body);
            return candidates
                .into_iter()
                .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
                .chain(lambda_words)
                .collect();
        }
    }

    lambda_words
}

/// Scan a `Call` / `Barrier` head word plus its non-body argument words into
/// `vars_found`. `Body`-role args (loop / `if` / `catch` scripts) are skipped —
/// they are lowered into their own CFG blocks. Extracted from [`uses_of`].
pub(super) fn scan_command_words(
    command: &str,
    canonical_command: Option<&str>,
    args: &[String],
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    out: &mut ClassifiedUses,
) {
    let registry = selection.registry;
    out.substituted
        .extend(selection.word_reads(scanner, command));
    let lookup = canonical_command.unwrap_or(command);
    let resolution = tokens.and_then(|tokens| selection.resolve(tokens));
    let facts = match resolution.as_ref() {
        Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) => {
            Some(facts.as_ref())
        }
        _ => None,
    };
    let effective = tokens.and_then(crate::registry_invocation::effective_command_words);
    let mut body_indices = HashSet::new();
    let mut name_role_braced = HashSet::new();
    let mut in_frame_braced = HashSet::new();
    if let Some(source) = tokens.and_then(|tokens| selection.source_structure(tokens)) {
        for (written, role) in source.written_roles() {
            if matches!(
                role,
                tcl_registry::ArgRole::VarWrite | tcl_registry::ArgRole::VarRead
            ) && tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(written))
            {
                name_role_braced.insert(written);
            }
        }
    }
    if let (Some(facts), Some(effective)) = (facts, effective.as_ref()) {
        for &(index, role) in &facts.arg_roles {
            let Some(written) =
                effective.written_argument(facts.argument_offset + usize::from(index))
            else {
                continue;
            };
            if matches!(
                role,
                tcl_registry::ArgRole::VarWrite | tcl_registry::ArgRole::VarRead
            ) && tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(written))
            {
                name_role_braced.insert(written);
            }
            if role.braced_word_evaluated_in_frame() {
                if role == tcl_registry::ArgRole::Body
                    && (facts.body_kind == tcl_registry::BodyKind::Structural
                        || tokens.is_some_and(|tokens| tokens.evaluated_body().is_some()))
                {
                    body_indices.insert(written);
                } else {
                    in_frame_braced.insert(written);
                }
            }
        }
    } else if tokens.is_none() && selection.standalone {
        body_indices.extend(structural_body_indices(lookup, args, tokens, registry));
        let arguments: Vec<_> = args.iter().map(String::as_str).collect();
        in_frame_braced.extend(registry.arg_indices_evaluated_in_frame(lookup, &arguments));
    }
    let described = if tokens.is_some() {
        facts.is_some() || !selection.standalone
    } else {
        selection.standalone && registry_describes(registry, lookup)
    };
    for (idx, arg) in args.iter().enumerate() {
        if body_indices.contains(&idx) || name_role_braced.contains(&idx) {
            continue;
        }
        let braced = tokens.is_some_and(|t| t.arg_is_braced_literal(idx));
        for name in selection.word_reads(scanner, arg) {
            let class = braced_word_class(&BracedWordSite {
                braced,
                evaluated_in_frame: in_frame_braced.contains(&idx),
                described,
                word: arg,
                name: &name,
                selection,
                tokens,
                written_argument: idx,
                config: scanner.lexer_config(),
            });
            match class {
                UseClass::Quoted => out.quoted.insert(name),
                UseClass::Substituted => out.substituted.insert(name),
                // `braced_word_class` classifies a *word*, and a word is
                // never a name argument — those never reach the scanner.
                UseClass::Name => out.by_name.insert(name),
            };
        }
    }
    scan_nested_substitution_words(
        tokens,
        scanner,
        selection,
        &mut out.substituted,
        &mut out.by_name,
    );
}

/// Record the reads inside a nested `[cmd …]`'s brace-quoted words that the
/// nested command evaluates in *this* frame.
///
/// The loop above applies exactly this rule to a **statement's** words: a
/// braced word in an [`ArgRole`](tcl_registry::ArgRole) position whose
/// `braced_word_evaluated_in_frame` holds is [`UseClass::Substituted`] — "a
/// genuine read, here and now" — which is why `expr {$x + 1}` written as a
/// statement records `x`.
///
/// A nested substitution never becomes a statement, so its words never
/// reached that rule, and `$x` in `puts [expr {$x + 1}]` was recorded
/// nowhere: [`VarReferenceScanner::scan_word`] correctly declines to
/// substitute inside the braces (the *command* parser does not), and nothing
/// then asked `expr` whether it substitutes them itself (it does). The read
/// vanished from `SsaStatement::uses`, and with it from every consumer of
/// that map — SCCP, taint, type inference, def-use, GVN, interval bounds and
/// the shimmer detectors. It was observable in taint, where only a pair of
/// braces separated the two spellings:
///
/// ```tcl
/// eval [expr $tainted]      ;# T100
/// eval [expr {$tainted}]    ;# nothing, before this
/// expr {$tainted}           ;# T100 again, once it is a statement
/// ```
///
/// So this applies the same registry-driven rule at the nested positions,
/// rather than teaching any consumer about substitutions. A braced word the
/// nested command merely carries as data is left to the conservative `Quoted`
/// handling the enclosing word already got.
///
/// Writes only substituted names — a word the nested command evaluates in this
/// frame is a genuine read — so it takes that half directly rather than the
/// whole [`ClassifiedUses`], which lets the assignment path (whose collector
/// *is* the substituted set) share the one owner.
///
/// # Why `Expr` only, and not every in-frame role
///
/// [`ArgRole::braced_word_evaluated_in_frame`](tcl_registry::ArgRole::braced_word_evaluated_in_frame)
/// answers for `Body` as well as `Expr`, and for a **statement** that is right:
/// `foreach x $l {…}` runs its body in this frame, and the lowerer gives that
/// body its own CFG block, so the loop variable is *defined* where the body's
/// reads are seen.
///
/// A nested substitution gets no such block — that is the representational gap
/// this whole module works around — so a nested body's reads would arrive with
/// none of its own definitions:
///
/// ```tcl
/// set r [join [lmap x $lines {string toupper $x}] ","]
/// ```
///
/// scanning the `lmap` body records a read of `x` that nothing in the frame
/// ever writes, and `W210 read before it is set` fires on the loop variable.
/// `Expr` is the role that both substitutes here *and* binds nothing of its
/// own, which is exactly what the `[expr {$x + 1}]` case this exists for
/// needs; `Body` is left to the conservative handling until a nested
/// substitution is lowered as a command.
pub(super) fn scan_nested_substitution_words(
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    substituted: &mut BTreeSet<String>,
    by_name: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let config = scanner.lexer_config();
    if !selection.standalone
        && tokens.is_none_or(|tokens| selection.token_metadata(tokens).is_none())
    {
        return;
    }
    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        for evaluation in
            crate::word_subst::entered_expression_evaluations(tokens, config, registry)
        {
            let config = config.with_grammar(evaluation.grammar());
            let names = if scanner.element_qualified() {
                evaluation
                    .expression()
                    .vars_element_qualified_with_config(config)
            } else {
                evaluation.expression().vars_with_config(config)
            };
            substituted.extend(names);
        }
    }
    for lifted in crate::word_subst::lifted_calls(tokens, config) {
        if let Some(child) = lifted.tokens.as_ref()
            && child.source_binding.is_some()
        {
            scan_selected_expression_roles(child, scanner, selection, substituted);
            if let Some(invocation) = selection.normal(child) {
                for (index, role) in invocation.variable_roles() {
                    if (role == tcl_registry::ArgRole::VarRead
                        || (role == tcl_registry::ArgRole::VarWrite
                            && invocation
                                .variable_traits()
                                .contains(tcl_registry::Traits::READS_BEFORE_WRITE)))
                        && let Some(name) = invocation.argument_literal(index)
                    {
                        by_name.insert(
                            tcl_syntax::naming::normalise_var_name_braced(&name, true).to_owned(),
                        );
                    }
                }
            }
            continue;
        }
        if !selection.standalone {
            continue;
        }
        let arg_strs: Vec<&str> = lifted.args.iter().map(String::as_str).collect();
        let in_frame =
            registry.arg_indices_for_role(&lifted.command, &arg_strs, tcl_registry::ArgRole::Expr);
        for idx in in_frame {
            // Only a braced word needs recovering. An unbraced one already
            // substitutes at the command level, so the enclosing word's own
            // scan has seen it. Which word is braced is the segmenter's
            // answer, not a `{`/`}` test over the argument text: `{a}{b}` and
            // a word merely *containing* braces are not brace-quoted literals.
            let braced = match lifted.arg_words.get(idx) {
                Some(WordExpr::BracedLiteral { text, .. }) => text.as_str(),
                // The structure is only absent when the word recovery
                // declined the substitution's shape, and then there is
                // nothing to recover from it either.
                _ => continue,
            };
            substituted.extend(selection.word_reads(scanner, braced));
        }
    }
}

// Non-Expr selected handlers may also declare expression arguments. Their
// possible read roles remain separate from the entered Expr evaluator receipt.
pub(in super::super) fn scan_selected_expression_roles(
    tokens: &CommandTokens,
    scanner: &VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    substituted: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let Some(metadata) = selection.token_metadata(tokens) else {
        return;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return;
    };
    let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
        return;
    };
    if let Some(evaluation) = binding.conditional_expression_evaluation(registry, tokens) {
        let config = config.with_grammar(evaluation.lexer_grammar());
        let names = if scanner.element_qualified() {
            evaluation.tree().vars_element_qualified_with_config(config)
        } else {
            evaluation.tree().vars_with_config(config)
        };
        substituted.extend(names);
    }
    if binding.runtime_reachability() != crate::command_binding::SourceRuntimeReachability::Reached
    {
        return;
    }
    let Some(invocation) =
        crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )
    else {
        return;
    };
    if invocation.facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
    {
        return;
    }
    let Some(dialect) = binding.variable_context.invocation_dialect else {
        return;
    };
    let parser = dialect.expression_parse_context(None);
    let config = config.with_grammar(parser.lexer_grammar);
    for (index, role) in invocation.written_argument_roles() {
        if role != tcl_registry::ArgRole::Expr {
            continue;
        }
        let Some(WordExpr::BracedLiteral { text, .. }) = tokens.words().get(index + 1) else {
            continue;
        };
        let expression = crate::expr_parser::parse_expr_with_syntax_context(text, &parser);
        let names = if scanner.element_qualified() {
            expression.vars_element_qualified_with_config(config)
        } else {
            expression.vars_with_config(config)
        };
        substituted.extend(names);
    }
}

/// Whether the registry describes `name` — accepting a space-joined
/// *ensemble subcommand* spelling (`dict for`, `dict map`) as well as a plain
/// command name.
///
/// The CFG's synthetic loop header names itself that way
/// (`cfg_builder::cfg_lower::lower_foreach`), and `registry.get` keys only on
/// whole command names, so a plain `get` would report a `dict for` header as
/// *unclassified* and read its braced value word as a substitution.  Mirrors
/// the base/sub split `shimmer::use_site::foreach_header_expected_type`
/// already does for the same statement.
fn registry_describes(registry: &CommandRegistry, name: &str) -> bool {
    if registry.get(name).is_some() {
        return true;
    }
    name.split_once(' ').is_some_and(|(base, sub)| {
        registry
            .get(base)
            .is_some_and(|spec| spec.resolve_subcommand(sub).is_some())
    })
}

/// One `(argument word, name found in it)` pair, with the facts
/// [`braced_word_class`] decides on.
pub(in super::super) struct BracedWordSite<'a> {
    /// The word is a brace-quoted literal — Tcl substitutes nothing in it.
    pub(in super::super) braced: bool,
    /// The registry gives this position a role whose word the callee
    /// re-evaluates in the *calling* frame (`Body` / `Expr`).
    pub(in super::super) evaluated_in_frame: bool,
    /// The registry has a `CommandSpec` for this command at all.
    pub(in super::super) described: bool,
    /// The word's text (braces already stripped).
    pub(in super::super) word: &'a str,
    /// The name the scan found inside it.
    pub(in super::super) name: &'a str,
    /// Actual availability and the explicit standalone utility boundary.
    pub(in super::super) selection: SsaInvocationContext<'a>,
    /// Original whole invocation, including the body word's written ordinal.
    pub(in super::super) tokens: Option<&'a CommandTokens>,
    pub(in super::super) written_argument: usize,
    /// The document's lexing configuration — the word is re-segmented as a
    /// script below, and that re-read must draw the same word boundaries the
    /// document's own grammar draws.
    pub(in super::super) config: tcl_lexer::LexerConfig,
}

/// How this statement consumes `site.name`.
///
/// Tcl substitutes nothing inside `{…}`, so a braced word is never read *at*
/// the call site. What the word then **is** falls into three kinds, and the
/// answer differs per kind:
///
/// - **script, this frame** — `expr {$a + $b}`, `if {$c} …`: the callee
///   re-evaluates the text where the caller's variables are in scope, so the
///   name really is read here. [`UseClass::Substituted`].
/// - **data** — a braced word of a command the registry *describes*, at a
///   role that never evaluates it: `puts {$y}`, `string match {$pat*} …`,
///   `lsort -command {cmp $x}`. The registry is the authority that nothing
///   here evaluates the word in this frame, so there is no read.
///   [`UseClass::Quoted`].
/// - **unclassified** — a braced word of a command the registry does *not*
///   describe: a user proc, an unknown definer. It may be a script, and if it
///   is it may run in this frame — a wrapper that hands it to an
///   `uplevel`-ing worker does exactly that, and tclsh then errors on an
///   unset name — so the read stands. **Unless** the word binds the name
///   itself: then the read is of that script's own local whichever frame it
///   runs in, which is the shape an un-hooked definer body takes.
pub(in super::super) fn braced_word_class(site: &BracedWordSite<'_>) -> UseClass {
    if !site.braced || site.evaluated_in_frame {
        return UseClass::Substituted;
    }
    if site.described || braced_word_owns_name(site) {
        return UseClass::Quoted;
    }
    UseClass::Substituted
}

/// An ownership exclusion consumes a complete original body query. MAY names
/// in an opaque residual cannot prove that a read belongs to that body.
pub(in super::super) fn braced_word_owns_name(site: &BracedWordSite<'_>) -> bool {
    if site.selection.standalone {
        return crate::script_binds::script_binds_name(
            site.word,
            site.name,
            crate::script_binds::Ownership::Bindings,
            site.selection.registry,
            site.config,
        );
    }
    let Some(tokens) = site.tokens else {
        return false;
    };
    let Some(ownership) = crate::script_binds::original_literal_body_ownership(
        site.selection.registry,
        site.selection.token_metadata(tokens),
        tokens,
        site.written_argument,
        crate::script_binds::Ownership::Bindings,
    ) else {
        return false;
    };
    let root = tcl_syntax::naming::split_element_ref(site.name).map_or(site.name, |(root, _)| root);
    !ownership.opaque && ownership.names.iter().any(|name| name == root)
}
