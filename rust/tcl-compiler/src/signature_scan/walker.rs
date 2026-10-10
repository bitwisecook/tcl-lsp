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

//! Top-level walker for the `signature_scan` module.
//!
//! Walks segmented commands and dispatches them to per-command
//! handlers in [`super::handlers`]. Definer commands — the class
//! systems and `proc` — are dispatched from registry data
//! ([`dispatch_definer`]: `definition_body` grammar family + traits),
//! never a hardcoded name list. Body recursion into braced
//! scripts (proc bodies, namespace-eval bodies, structured-command
//! branches) lives here too — it must not depend on the IR
//! lowering pass, which is the whole reason the `signature_scan`
//! module exists.
//!
//! Public-to-the-module entry points:
//!
//! - [`scan`] — the main walker; called by
//!   [`super::extract_signatures`] for the top-level source and
//!   recursively by [`maybe_recurse_body`] for braced bodies.
//! - [`maybe_recurse_body`] — gates body recursion on `Str`
//!   (braced) tokens; called from the registry-dispatched namespace-eval arm,
//!   `handle_clause_bodies` (`if`, `try`: the clause plan's script words) and
//!   `handle_catch` here.
//! - [`scan_factory_candidates`] — secondary walker called from
//!   `handle_proc`; only collects four-token factory candidates and
//!   recurses into structural-control bodies via
//!   `scan_factory_structural`.

use std::collections::HashSet;

use tcl_lexer::{Token, TokenType};
use tcl_registry::SubCommand;
use tcl_registry::Traits;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::DefinerFamily;
use tcl_registry::hooks::{AnalyserHookId, LoweringHookId};
use tcl_registry::{ClausePlan, CommandSpec, SubCommand};

use super::ctx::ScanCtx;
use super::handlers;
use super::types::SignatureCommandInvocation;
use crate::segmenter::{
    SegmentedCommand, segment_commands_with_offset_and_config,
    segment_commands_with_recovery_and_config,
};

/// Walk *source* as a Tcl script, emitting records for every command
/// the dispatcher recognises.
///
/// When `body_token` is `Some`, the spans on every record are
/// relocated into the outer source buffer's offset space (the body
/// token's content position is used as the base offset). Body
/// recursion never runs segmenter-level error recovery — recovery
/// only fires at the top-level entry point, where the segmented
/// stream feeds workspace-index consumers that must not be silently
/// truncated by a single unclosed delimiter.
pub(super) fn scan_in_context(
    source: &str,
    body_token: Option<Token>,
    namespace: &super::scope::SignatureNamespaceScope,
    conditional: bool,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    let label = namespace.display().unwrap_or_default();
    let previous = ctx.namespace_scope.replace(namespace.clone());
    scan(source, body_token, &label, conditional, known_commands, ctx);
    ctx.namespace_scope = previous;
}

pub(super) fn scan(
    source: &str,
    body_token: Option<Token>,
    ns_prefix: &str,
    conditional: bool,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    let commands = match body_token {
        None => segment_commands_with_recovery_and_config(source, known_commands, ctx.config),
        Some(tok) => {
            let base = tok.span.start() + u32::from(tok.content_offset);
            segment_commands_with_offset_and_config(source, base, ctx.config)
        }
    };
    for cmd in commands {
        if cmd.is_partial || cmd.argv.is_empty() {
            continue;
        }
        let head = cmd.name();
        if head.is_empty() {
            continue;
        }
        let original_words = ctx.original_command_words(&cmd).unwrap_or_default();
        let previous_words = std::mem::replace(&mut ctx.original_words, original_words);
        // Argument count for cross-file arity (Task 6): words after the head, or
        // `None` when any argument is `{*}`-expanded (runtime count unknown).
        let arg_count = if cmd
            .expand_word
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .skip(1)
            .copied()
            .any(|e| e)
        {
            None
        } else {
            Some(cmd.argv.len().saturating_sub(1))
        };
        ctx.result
            .command_invocations
            .push(SignatureCommandInvocation {
                original_name_input: ctx
                    .original_name_key(cmd.argv[0].span)
                    .map(super::scope::SignatureSourceNameInput::OriginalWord),
                ..SignatureCommandInvocation::written(head.to_owned(), cmd.argv[0].span, arg_count)
            });
        // Record command-prefix callback heads (`lsort -command cb`, `trace
        // add … cb`, …) as their own invocations so background-scanned files
        // feed find-references / call-hierarchy / usage counts / callback
        // arity through the same substrate as ordinary calls.
        record_command_prefix_invocations(&cmd, ctx);
        let texts = &cmd.texts;
        let argv = &cmd.argv;
        let handled = record_selected_alias(&cmd, ns_prefix, ctx)
            || resolve_scan_dispatch(ctx.registry, head, texts).is_some_and(|dispatch| {
                if dispatch.analyser == Some(AnalyserHookId::NamespaceEval)
                    && cmd.single_token_word.get(2) != Some(&true)
                {
                    return true;
                }
                if dispatch.analyser == Some(AnalyserHookId::Source)
                    && cmd
                        .expand_word
                        .as_ref()
                        .is_some_and(|words| words.iter().any(|&word| word))
                {
                    return true;
                }
                dispatch_signature_handler(
                    dispatch,
                    texts,
                    argv,
                    ns_prefix,
                    conditional,
                    known_commands,
                    ctx,
                )
            });
        if !handled && !dispatch_definer(head, texts, argv, &cmd.single_token_word, ns_prefix, ctx)
        {
            if let Some(namespace) = ctx
                .current_namespace(ns_prefix)
                .and_then(|scope| scope.source_spelling(ctx.name_policy()))
            {
                handlers::maybe_handle_import_wrapper(
                    head,
                    texts,
                    argv,
                    &namespace,
                    &mut ctx.result,
                );
            }
            handlers::maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx);
        }
        ctx.original_words = previous_words;
    }
}

fn record_selected_alias(cmd: &SegmentedCommand, namespace: &str, ctx: &mut ScanCtx<'_>) -> bool {
    use tcl_registry::{
        AliasTargetLookup, CommandBindingTransition, InvocationWord, InvocationWords,
    };
    let Some(registry) = ctx.registry else {
        return false;
    };
    let words: Vec<_> = cmd
        .texts
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, text)| {
            if cmd
                .expand_word
                .as_ref()
                .and_then(|expanded| expanded.get(index))
                .copied()
                .unwrap_or(false)
            {
                InvocationWord::Expanded
            } else if cmd.single_token_word.get(index) == Some(&true)
                && cmd
                    .argv
                    .get(index)
                    .is_some_and(|token| matches!(token.kind, TokenType::Str | TokenType::Esc))
            {
                InvocationWord::Literal(text)
            } else {
                InvocationWord::Dynamic
            }
        })
        .collect();
    let transitions = crate::alias::command_table_transitions_for_words(
        registry,
        InvocationWords::structured(InvocationWord::Literal(cmd.name()), &words),
    );
    let Some(CommandBindingTransition::Alias {
        source_interpreter,
        alias,
        target_interpreter,
        target,
        arguments,
        target_lookup,
    }) = transitions.command_bindings().next()
    else {
        return false;
    };
    if !crate::alias::is_current_interpreter(source_interpreter)
        || !crate::alias::is_current_interpreter(target_interpreter)
    {
        return true;
    }
    let alias_span = alias
        .argument_index()
        .and_then(|ordinal| cmd.argv.get(ordinal + 1))
        .map(|token| token.span);
    let (Some(alias), Some(target), Some(scope)) = (
        alias.literal(),
        target.literal(),
        ctx.current_namespace(namespace),
    ) else {
        return true;
    };
    let Some((qualified_name, source_name)) = ctx.publication_name(
        &scope,
        alias,
        tcl_syntax::naming::NativeNamePurpose::AliasPublication,
        match alias_span {
            Some(span) => span,
            None => return true,
        },
    ) else {
        return true;
    };
    let extras: Option<Vec<_>> = arguments
        .iter()
        .map(|argument| argument.literal().map(str::to_owned))
        .collect();
    let Some(extras) = extras else {
        return true;
    };
    let target = match target_lookup {
        AliasTargetLookup::Global => {
            super::types::SignatureCommandAliasTarget::WrittenGlobal(target.to_owned())
        }
        AliasTargetLookup::CallerNamespace => {
            super::types::SignatureCommandAliasTarget::WrittenCaller(target.to_owned())
        }
    };
    ctx.result.command_aliases.insert(
        qualified_name.clone(),
        super::types::SignatureCommandAlias {
            source_name,
            qualified_name,
            target,
            extras,
        },
    );
    true
}

#[derive(Clone, Copy)]
struct ResolvedScanDispatch<'r> {
    subcommand: Option<&'r SubCommand>,
    analyser: Option<AnalyserHookId>,
    lowering: Option<LoweringHookId>,
}

impl ResolvedScanDispatch<'_> {
    /// The clause plan of the scanned call: the resolved descriptor's clause
    /// grammar walked over `texts` after the head (and after the subcommand
    /// word), in post-head coordinates. A background scan carries no
    /// document point, so every row is available, as the dispatch itself is
    /// dialect-blind.
    fn clause_plan_for(&self, texts: &[String]) -> Option<ClausePlan> {
        let args: Vec<&str> = texts.iter().skip(1).map(String::as_str).collect();
        match self.subcommand {
            Some(sub) => sub
                .clause_plan(args.get(1..).unwrap_or_default(), None)
                .map(|plan| plan.offset_by(1)),
            None => self.spec.clause_plan(&args, None),
        }
    }
}

/// The script words of a clause-carrying call, in source order and in the
/// scanned command's own coordinates (the head is word 0): each clause's body
/// word, except one that is the grammar's fall-through marker — it runs
/// another clause's body and is no script of its own.
fn clause_body_words(plan: &ClausePlan) -> impl Iterator<Item = usize> + '_ {
    plan.clauses
        .iter()
        .enumerate()
        .filter(|&(index, _)| !plan.falls_through(index))
        .filter_map(|(_, clause)| clause.operand(ArgRole::Body))
        .map(|word| word + 1)
}

fn resolve_scan_dispatch<'r>(
    registry: Option<&'r tcl_registry::CommandRegistry>,
    head: &str,
    texts: &[String],
) -> Option<ResolvedScanDispatch<'r>> {
    let spec = registry?.get(head)?;
    let subcommand = texts.get(1).and_then(|word| spec.resolve_subcommand(word));
    Some(ResolvedScanDispatch {
        subcommand,
        analyser: subcommand
            .and_then(|sub| sub.analyser_hook)
            .or(spec.analyser_hook),
        lowering: subcommand
            .and_then(|sub| sub.lowering_hook)
            .or(spec.lowering_hook),
    })
}

fn dispatch_signature_handler(
    dispatch: ResolvedScanDispatch<'_>,
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    conditional: bool,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx<'_>,
) -> bool {
    let checked_namespace = ctx
        .current_namespace(ns_prefix)
        .and_then(|scope| scope.source_spelling(ctx.name_policy()));
    if matches!(
        dispatch.analyser,
        Some(
            AnalyserHookId::NamespaceImport
                | AnalyserHookId::NamespaceForget
                | AnalyserHookId::Source
        )
    ) && checked_namespace.is_none()
    {
        return true;
    }
    let compatibility_namespace = checked_namespace.as_deref().unwrap_or(ns_prefix);
    match dispatch.analyser {
        Some(AnalyserHookId::NamespaceEval) => {
            handlers::handle_namespace_eval(
                texts,
                argv,
                ns_prefix,
                conditional,
                known_commands,
                ctx,
            );
        }
        Some(AnalyserHookId::NamespaceImport) => handlers::handle_namespace_import_in_context(
            texts,
            argv,
            compatibility_namespace,
            ctx.name_policy(),
            ctx.namespace_scope.as_ref(),
            dispatch.subcommand,
            &mut ctx.result,
        ),
        Some(AnalyserHookId::NamespaceForget) => handlers::handle_namespace_forget_in_context(
            texts,
            argv,
            compatibility_namespace,
            ctx.name_policy(),
            ctx.namespace_scope.as_ref(),
            dispatch.subcommand,
            &mut ctx.result,
        ),
        Some(AnalyserHookId::PackageRequire) => {
            record_package_require(texts, argv, conditional, dispatch.subcommand, ctx);
        }
        Some(AnalyserHookId::PackageProvide) if texts.len() >= 3 => {
            record_package_provide(texts, argv, conditional, ctx);
        }
        Some(AnalyserHookId::PackageIfneeded) if texts.len() >= 5 => {
            record_package_ifneeded(texts, argv, ctx);
        }
        Some(AnalyserHookId::Source) => {
            let dialect = ctx
                .registry
                .and_then(tcl_registry::CommandRegistry::profile)
                .map(tcl_registry::InvocationDialect::of_profile);
            handlers::handle_source(
                texts,
                argv,
                compatibility_namespace,
                dialect,
                &mut ctx.result,
            );
        }
        Some(AnalyserHookId::InterpAlias) => {
            handlers::handle_interp_alias(texts, ctx.name_policy(), &mut ctx.result);
        }
        Some(AnalyserHookId::Rename) => {
            record_command_rename(texts, argv, ns_prefix, ctx);
        }
        Some(AnalyserHookId::Catch) => {
            handle_catch(texts, argv, ns_prefix, known_commands, ctx);
        }
        // `if` and `try` carry no analyser hook — their
        // clause-carrying bodies walk through the lowering hook they still
        // have instead.
        _ if matches!(
            dispatch.lowering,
            Some(LoweringHookId::If | LoweringHookId::Try)
        ) =>
        {
            handle_clause_bodies(dispatch, texts, argv, ns_prefix, known_commands, ctx);
        }
        // `lappend` carries no analyser hook either; a command that
        // appends list elements to its target variable is exactly `lappend`
        // (`append` writes a string, not list elements), the same registry
        // fact the full analyser's own `auto_path` handling reads.
        _ if matches!(
            dispatch.spec.var_elements_effect,
            Some(tcl_registry::VarElementsEffect::AppendsListElements { .. })
        ) =>
        {
            handlers::handle_auto_path(texts, argv, &mut ctx.result);
        }
        // `set` carries no analyser hook either: a
        // command whose declared semantics stores its value word into the
        // variable it names assigns the search path, the registry fact the
        // full analyser's `bind_value_word_assignment` reads.
        _ if tcl_registry::value_transfer::resolve_semantics(
            dispatch.spec,
            dispatch.subcommand,
            None,
        )
        .writes_value_word() =>
        {
            handlers::handle_auto_path(texts, argv, &mut ctx.result);
        }
        _ => return false,
    }
    true
}

fn record_command_rename(texts: &[String], argv: &[Token], ns_prefix: &str, ctx: &mut ScanCtx<'_>) {
    if let (Some(old), Some(new), Some(scope)) = (
        texts.get(1),
        texts.get(2).filter(|new| !new.is_empty()),
        ctx.current_namespace(ns_prefix),
    ) && let Some((qualified_name, source_name)) = ctx.publication_name(
        &scope,
        new,
        tcl_syntax::naming::NativeNamePurpose::RenameDestination,
        argv[2].span,
    ) {
        ctx.result.renames.insert(
            qualified_name.clone(),
            super::types::SignatureRename {
                source_name,
                qualified_name,
                target: old.clone(),
            },
        );
    }
}

fn record_package_require(
    texts: &[String],
    argv: &[Token],
    conditional: bool,
    subcommand: Option<&SubCommand>,
    ctx: &mut ScanCtx<'_>,
) {
    let before = ctx.result.package_requires.len();
    handlers::handle_package_require(texts, argv, conditional, subcommand, &mut ctx.result);
    if let Some(record) = ctx.result.package_requires.get(before) {
        let original = ctx
            .original_name_key(record.range)
            .map(super::scope::SignatureSourceNameInput::OriginalWord)
            .map(super::original_name::SourcePackageName::from_input);
        let name_ordinal = argv.iter().position(|token| token.span == record.range);
        let requirements = name_ordinal.map_or_else(Vec::new, |name| {
            argv[name + 1..]
                .iter()
                .map(|token| {
                    ctx.original_name_key(token.span)
                        .map(super::scope::SignatureSourceNameInput::OriginalWord)
                })
                .collect()
        });
        ctx.result.package_requires[before].original_name = original;
        ctx.result.package_requires[before].original_requirements = requirements;
    }
}

fn record_package_provide(
    texts: &[String],
    argv: &[Token],
    conditional: bool,
    ctx: &mut ScanCtx<'_>,
) {
    let original_name = ctx
        .original_name_key(argv[2].span)
        .map(super::scope::SignatureSourceNameInput::OriginalWord)
        .map(super::original_name::SourcePackageName::from_input);
    ctx.result
        .package_provides
        .push(crate::analyser::types::PackageProvide {
            original_name,
            name: texts[2].clone(),
            version: texts.get(3).cloned(),
            range: argv[0].span,
            conditional,
        });
}

fn record_package_ifneeded(texts: &[String], argv: &[Token], ctx: &mut ScanCtx<'_>) {
    let original_name = ctx
        .original_name_key(argv[2].span)
        .map(super::scope::SignatureSourceNameInput::OriginalWord)
        .map(super::original_name::SourcePackageName::from_input);
    ctx.result
        .package_ifneededs
        .push(crate::analyser::types::PackageIfneeded {
            original_name,
            name: texts[2].clone(),
            version: texts[3].clone(),
            range: argv[0].span,
        });
}

/// Dispatch `head` to a definer handler when its registry spec marks it as a
/// class or procedure definer, returning whether it was claimed.
///
/// Recognition is registry data, never a name list: a spec carrying a
/// [`tcl_registry::definer::DefinitionBodyGrammar`] dispatches on the
/// grammar's [`DefinerFamily`] — mirroring the analyser's OO handlers — and a
/// spec carrying [`Traits::DEFINES_PROCEDURE`] (with no definition body) is a
/// `proc`-shaped procedure definer, so a new definer of an existing family is
/// picked up the moment its spec carries the grammar. A `::`-qualified
/// spelling resolves through [`tcl_registry::CommandRegistry::get`]'s
/// canonical leading-`::` fallback to the bare name. A `true` return means
/// the generic import-wrapper / factory-candidate handlers must not run,
/// matching the former dedicated match arms.
fn dispatch_definer(
    head: &str,
    texts: &[String],
    argv: &[Token],
    single_token_word: &[bool],
    ns_prefix: &str,
    ctx: &mut ScanCtx,
) -> bool {
    let Some(spec) = ctx.registry.and_then(|r| r.get(head)) else {
        return false;
    };
    if let Some(grammar) = spec.definition_body {
        let mut definitions = super::types::SignatureScanResult::default();
        let handled = match grammar.family {
            // Every stock `TclOO` metaclass creates a class via the same
            // `METACLASS create NAME ?BODY?` interface — `oo::configurable`
            // (property-bearing), `oo::abstract`, and `oo::singleton`
            // included, so a `[Pin new]` on an `oo::configurable` class is
            // typed as an object like any other.
            DefinerFamily::TclOo if spec.traits.contains(Traits::IS_OO_METACLASS) => {
                if let Some(method) = texts
                    .get(1)
                    .and_then(|word| ctx.registry?.exported_manufacturer_method(head, word))
                {
                    handlers::handle_oo_class(texts, argv, method, ns_prefix, &mut definitions);
                }
                true
            }
            // `oo::define` / `oo::objdefine` share the `TclOO` grammar but
            // extend an existing class rather than create one — and the
            // declaration-only families manufacture nothing at all: a
            // `.tclspec` body describes commands and a `.sslictcl` body
            // describes TLS facts, neither creating anything. None records a
            // definition here; the ordinary scan continues past them.
            DefinerFamily::TclOo | DefinerFamily::SpecTcl | DefinerFamily::SslicTcl => false,
            // snit types/widgets create instances via `Name create obj` /
            // `Name %AUTO%` / a widget's `Name .path`, so record them as
            // classes to type those constructors' receivers (same shape as
            // itcl).
            DefinerFamily::Snit => {
                handlers::handle_snit_type(texts, argv, ns_prefix, &mut definitions);
                true
            }
            DefinerFamily::Itcl => {
                handlers::handle_itcl_class(texts, argv, ns_prefix, &mut definitions);
                true
            }
            // `class NAME ?BASES? VARS` — the variable dictionary is the last
            // word, whether or not base classes are named.
            DefinerFamily::JimClass => {
                handlers::handle_jim_class(texts, argv, ns_prefix, &mut definitions);
                true
            }
        };
        if let Some(scope) = ctx.current_namespace(ns_prefix) {
            for mut declaration in definitions.classes.into_values() {
                let Some(index) = argv
                    .iter()
                    .position(|token| token.span == declaration.name_range)
                else {
                    continue;
                };
                if single_token_word.get(index) != Some(&true)
                    || !matches!(argv[index].kind, TokenType::Str | TokenType::Esc)
                {
                    continue;
                }
                let Some((qualified, source_name)) = ctx.publication_name(
                    &scope,
                    &texts[index],
                    if grammar.family == DefinerFamily::TclOo {
                        tcl_syntax::naming::NativeNamePurpose::OoObjectPublication
                    } else {
                        tcl_syntax::naming::NativeNamePurpose::CommandPublication
                    },
                    argv[index].span,
                ) else {
                    continue;
                };
                declaration.qualified_name = qualified;
                declaration.source_name = source_name;
                declaration.name = declaration
                    .source_name
                    .as_ref()
                    .and_then(|name| name.slot().simple.try_utf8().ok())
                    .map_or_else(|| declaration.name.clone(), str::to_owned);
                ctx.result.class_declarations.push(declaration.clone());
                let ambiguous = ctx.result.class_declarations.iter().any(|other| {
                    other.qualified_name == declaration.qualified_name
                        && other.source_name != declaration.source_name
                });
                if ambiguous {
                    ctx.result.classes.remove(&declaration.qualified_name);
                } else {
                    ctx.result
                        .classes
                        .insert(declaration.qualified_name.clone(), declaration);
                }
            }
        }
        return handled;
    }
    // `tcl::OptProc name optlist body`: a real proc
    // definer, but `optlist` is never the arity-relevant param list the
    // way `proc`'s own second argument is — the runtime always installs
    // a plain `args` catch-all — so it needs its own handler rather than
    // `handle_proc`'s literal `parse_param_list(&texts[2])`, which would
    // record `optlist`'s own descriptor words as the recorded arity and
    // misreport a cross-file caller's true argument count.
    if spec.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::OptProc) {
        handlers::handle_opt_proc(texts, argv, ns_prefix, ctx);
        return true;
    }
    if spec.traits.contains(Traits::DEFINES_PROCEDURE) {
        handlers::handle_proc(texts, argv, single_token_word, ns_prefix, ctx);
        return true;
    }
    false
}

/// Record `cmd`'s [`tcl_registry::arg_role::ArgRole::CommandPrefix`] callback
/// heads (`lsort -command cb`, `trace add … cb`) as command invocations, so a
/// background-scanned file's callbacks feed find-references / call-hierarchy /
/// usage counts / callback-arity through the same substrate as ordinary calls.
///
/// No-op when the context carries no registry (focused unit tests) or the call
/// has no arguments.
fn record_command_prefix_invocations(cmd: &SegmentedCommand, ctx: &mut ScanCtx<'_>) {
    // naming.callback.lookup-scope-owner
    // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
    for written in 1..cmd.argv.len() {
        let Some(original) = ctx.original_callback_prefix(cmd, written) else {
            continue;
        };
        let Some(appended) = original.appended_arity() else {
            continue;
        };
        let Some((name, range)) = original.reported_source_head() else {
            continue;
        };
        let name = name.to_owned();
        let lookup = original.lookup().cloned();
        let name_input = original.name_input().clone();
        let baked = original.baked_argument_count();
        ctx.result
            .command_invocations
            .push(SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: Some(std::sync::Arc::new(original)),
                original_lookup: lookup,
                original_name_input: Some(name_input),
                lookup: crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
                name,
                range,
                resolved_qualified_name: None,
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc: None,
                callback_arity: Some(appended),
                callback_baked_args: baked,
                indirect: false,
                rename_safe: false,
                existence_probe: false,
                is_mathfunc_call: false,
                ensemble_dispatch: None,
            });
    }
}

/// Recurse into a braced body script.
///
/// Only `Str` (braced) bodies
/// can be statically analysed; substituted bodies (`$body`,
/// `[gen_body]`) cannot be re-segmented and are skipped.
pub(super) fn maybe_recurse_body(
    body_text: &str,
    body_tok: Token,
    ns_prefix: &str,
    conditional: bool,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    if body_tok.kind != TokenType::Str {
        return;
    }
    scan(
        body_text,
        Some(body_tok),
        ns_prefix,
        conditional,
        known_commands,
        ctx,
    );
}

// Body-recursion handlers
// Handlers for commands that recurse into braced bodies.

/// Recurse into every script word of a clause-carrying call (`if`'s bodies,
/// `try`'s protected body, handlers and `finally`), read from the call's
/// clause plan. Every recursed body is marked `conditional=true`: it is
/// branch-selected or guarded, so nothing it records dominates the code after
/// the command.
fn handle_clause_bodies(
    dispatch: ResolvedScanDispatch<'_>,
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    let Some(plan) = dispatch.clause_plan_for(texts) else {
        return;
    };
    for word in clause_body_words(&plan) {
        if let (Some(text), Some(tok)) = (texts.get(word), argv.get(word)) {
            maybe_recurse_body(text, *tok, ns_prefix, true, known_commands, ctx);
        }
    }
}

fn handle_catch(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    // `catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?` — only the first argument
    // is a body. Marked `conditional=true` since the body is guarded
    // (it could throw before reaching subsequent statements).
    if texts.len() < 2 {
        return;
    }
    maybe_recurse_body(&texts[1], argv[1], ns_prefix, true, known_commands, ctx);
}

/// Scan a proc body specifically for factory-wrapper candidate
/// calls.
///
/// Unlike [`scan`], this walker
/// only collects four-token `HEAD NAME ARGS BODY` shaped calls and
/// recurses into structural-control bodies (`if` / `catch` / `try`
/// / namespace-evaluation) that commonly wrap factory calls. It
/// deliberately does **not** emit nested proc / namespace-import
/// records — those would be incorrect because nested `proc`
/// statements inside a proc body only take effect when that proc
/// is invoked.
pub(super) fn scan_factory_candidates_in_context(
    body_text: &str,
    body_tok: Token,
    namespace: &super::scope::SignatureNamespaceScope,
    ctx: &mut ScanCtx,
) {
    let label = namespace.display().unwrap_or_default();
    let previous = ctx.namespace_scope.replace(namespace.clone());
    scan_factory_candidates(body_text, body_tok, &label, ctx);
    ctx.namespace_scope = previous;
}

pub(super) fn scan_factory_candidates(
    body_text: &str,
    body_tok: Token,
    ns_prefix: &str,
    ctx: &mut ScanCtx,
) {
    let base = body_tok.span.start() + u32::from(body_tok.content_offset);
    let commands = segment_commands_with_offset_and_config(body_text, base, ctx.config);
    for cmd in commands {
        if cmd.is_partial || cmd.argv.is_empty() {
            continue;
        }
        let head = cmd.name();
        if head.is_empty() {
            continue;
        }
        let original_words = ctx.original_command_words(&cmd).unwrap_or_default();
        let previous_words = std::mem::replace(&mut ctx.original_words, original_words);
        let texts = &cmd.texts;
        let argv = &cmd.argv;
        let structural = resolve_scan_dispatch(ctx.registry, head, texts).is_some_and(|dispatch| {
            if dispatch.analyser == Some(AnalyserHookId::NamespaceEval)
                && cmd.single_token_word.get(2) != Some(&true)
            {
                return true;
            }
            scan_factory_structural(dispatch, texts, argv, ns_prefix, ctx)
        });
        if !structural {
            handlers::maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx);
        }
        ctx.original_words = previous_words;
    }
}

/// Recurse into a structural command's braced bodies for factory
/// candidates only.
///
/// Same set of structural
/// commands and same body offsets as the main typed structural handlers
/// walkers, but the recursive call is `scan_factory_candidates`
/// (not `scan`) so only factory-shaped calls are collected.
fn scan_factory_structural(
    dispatch: ResolvedScanDispatch<'_>,
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    ctx: &mut ScanCtx,
) -> bool {
    if dispatch.analyser == Some(AnalyserHookId::NamespaceEval) && texts.len() >= 4 {
        let raw_ns = &texts[2];
        let Some(scope) = ctx.namespace_context(ns_prefix, raw_ns, argv[2].span) else {
            return true;
        };
        if argv[3].kind == TokenType::Str {
            scan_factory_candidates_in_context(&texts[3], argv[3], &scope, ctx);
        }
        return true;
    }
    if matches!(
        dispatch.lowering,
        Some(LoweringHookId::If | LoweringHookId::Try)
    ) {
        // The clause plan's script words — `if`'s bodies, `try`'s protected
        // body, handlers and `finally` — never a keyword walk.
        if let Some(plan) = dispatch.clause_plan_for(texts) {
            for word in clause_body_words(&plan) {
                if let (Some(text), Some(tok)) = (texts.get(word), argv.get(word))
                    && tok.kind == TokenType::Str
                {
                    scan_factory_candidates(text, *tok, ns_prefix, ctx);
                }
            }
        }
        return true;
    }
    if dispatch.analyser == Some(AnalyserHookId::Catch) {
        if texts.len() >= 2 && argv[1].kind == TokenType::Str {
            scan_factory_candidates(&texts[1], argv[1], ns_prefix, ctx);
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scan context wired to the shared default registry — definer
    /// dispatch (class definers, `proc`) is registry-driven, so walker
    /// tests carry one, matching the production `extract_signatures` path.
    fn registry_ctx() -> ScanCtx<'static> {
        ScanCtx {
            registry: Some(tcl_registry::model::ingress::static_context_for("").commands()),
            ..ScanCtx::default()
        }
    }

    #[test]
    fn top_level_proc_emits_invocation_and_record() {
        let mut ctx = registry_ctx();
        scan(
            "proc foo {} { set x 1 }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::foo"));
        // command_invocations should contain at least the top-level "proc"
        // invocation; the body's "set" stays uninvoked since handle_proc
        // does not recurse into the body.
        let names: Vec<&str> = ctx
            .result
            .command_invocations
            .iter()
            .map(|inv| inv.name.as_str())
            .collect();
        assert_eq!(names, ["proc"]);
    }

    #[test]
    fn multiple_handlers_dispatch_correctly() {
        let mut ctx = registry_ctx();
        ctx.registry = Some(tcl_registry::model::ingress::static_context_for("tcl8.6").commands());
        scan(
            "package require Tcl 8.6\nsource /abs/path.tcl\nproc bar {} {}",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert_eq!(ctx.result.package_requires.len(), 1);
        assert_eq!(ctx.result.package_requires[0].name, "Tcl");
        assert_eq!(ctx.result.source_targets.len(), 1);
        assert_eq!(ctx.result.source_targets[0].raw_path, "/abs/path.tcl");
        assert!(ctx.result.procs.contains_key("::bar"));
        assert_eq!(ctx.result.command_invocations.len(), 3);
    }

    #[test]
    fn custom_registry_hook_dispatches_without_a_command_spelling_branch() {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "background-load-script",
            arity: tcl_registry::Arity::exact(1),
            analyser_hook: Some(AnalyserHookId::Source),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let mut ctx = ScanCtx {
            registry: Some(&registry),
            ..ScanCtx::default()
        };
        scan(
            "background-load-script /opt/app/init.tcl",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );

        assert_eq!(ctx.result.source_targets.len(), 1);
        assert_eq!(ctx.result.source_targets[0].raw_path, "/opt/app/init.tcl");
    }

    #[test]
    fn qualified_heads_and_registry_subcommand_prefixes_keep_signature_facts() {
        let mut ctx = registry_ctx();
        scan(
            "::package req Tcl 8.6\n::namespace ev tools {::proc helper {} {}}",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );

        assert_eq!(ctx.result.package_requires.len(), 1);
        assert!(ctx.result.procs.contains_key("::tools::helper"));
    }

    #[test]
    fn namespace_eval_recurses_into_body() {
        let mut ctx = registry_ctx();
        scan(
            "namespace eval ns { proc inner {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::ns::inner"));
    }

    #[test]
    fn namespace_eval_absolute_rebases_prefix() {
        let mut ctx = registry_ctx();
        scan(
            "namespace eval outer { namespace eval ::abs { proc foo {} {} } }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        // The inner ::abs eval should rebase, not nest under outer.
        assert!(ctx.result.procs.contains_key("::abs::foo"));
    }

    #[test]
    fn handle_if_then_else_recurses_both_branches() {
        let mut ctx = registry_ctx();
        scan(
            "if {$x} { proc thenproc {} {} } else { proc elseproc {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::thenproc"));
        assert!(ctx.result.procs.contains_key("::elseproc"));
    }

    #[test]
    fn handle_if_elseif_chain_recurses_each_body() {
        let mut ctx = registry_ctx();
        scan(
            "if {$x} { proc a {} {} } elseif {$y} { proc b {} {} } elseif {$z} { proc c {} {} } else { proc d {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        for name in ["::a", "::b", "::c", "::d"] {
            assert!(ctx.result.procs.contains_key(name), "missing {name}");
        }
    }

    #[test]
    fn handle_if_explicit_then_keyword() {
        let mut ctx = registry_ctx();
        scan(
            "if {$x} then { proc thenproc {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::thenproc"));
    }

    #[test]
    fn handle_catch_braced_body() {
        let mut ctx = registry_ctx();
        scan(
            "catch { proc inner {} {} } result",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::inner"));
    }

    #[test]
    fn handle_catch_unbraced_body_skipped() {
        let mut ctx = registry_ctx();
        scan("catch $script", None, "", false, &HashSet::new(), &mut ctx);
        // No procs since the body cannot be statically analysed.
        assert!(ctx.result.procs.is_empty());
    }

    #[test]
    fn handle_try_with_finally() {
        let mut ctx = registry_ctx();
        scan(
            "try { proc tryproc {} {} } finally { proc finallyproc {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::tryproc"));
        assert!(ctx.result.procs.contains_key("::finallyproc"));
    }

    #[test]
    fn handle_try_with_on_handler() {
        let mut ctx = registry_ctx();
        scan(
            "try { proc tryproc {} {} } on error {res opts} { proc onproc {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::tryproc"));
        assert!(ctx.result.procs.contains_key("::onproc"));
    }

    #[test]
    fn handle_try_with_trap_handler() {
        let mut ctx = registry_ctx();
        scan(
            "try { proc tryproc {} {} } trap {ARITH DIVZERO} {res opts} { proc trapproc {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::tryproc"));
        assert!(ctx.result.procs.contains_key("::trapproc"));
    }

    fn extract_proc_body(src: &str) -> (String, Token) {
        // Pluck the proc body from the segmenter so we get a real
        // `Str` token with correct content_offset/span.
        let cmds = crate::segmenter::segment_commands(src);
        let proc_cmd = cmds.first().expect("one command");
        let body_tok = proc_cmd.argv[3];
        let span = body_tok.span;
        let inner = &src[span.start() as usize + 1..span.end() as usize - 1];
        (inner.to_string(), body_tok)
    }

    #[test]
    fn factory_walker_records_bare_candidate() {
        // DEFC's body argument must be a `Str` (braced) for the
        // candidate to register.
        let src = "proc factwrapper {a b c} { DEFC bar args {body} }";
        let (body, body_tok) = extract_proc_body(src);
        let mut ctx = registry_ctx();
        scan_factory_candidates(&body, body_tok, "", &mut ctx);
        assert_eq!(ctx.candidates.len(), 1);
    }

    #[test]
    fn factory_walker_recurses_into_if() {
        let src = "proc init {} { if {1} { DEFC bar args {body} } }";
        let (body, body_tok) = extract_proc_body(src);
        let mut ctx = registry_ctx();
        scan_factory_candidates(&body, body_tok, "", &mut ctx);
        assert_eq!(ctx.candidates.len(), 1);
    }

    #[test]
    fn factory_walker_recurses_into_try_finally() {
        let src = "proc init {} { try { DEFC a {x} {b} } finally { DEFC c {y} {d} } }";
        let (body, body_tok) = extract_proc_body(src);
        let mut ctx = registry_ctx();
        scan_factory_candidates(&body, body_tok, "", &mut ctx);
        assert_eq!(ctx.candidates.len(), 2);
    }

    #[test]
    fn class_definer_families_recognised_from_registry() {
        // TP guard: every class definer emits a class record via registry
        // dispatch.
        for (src, key) in [
            ("oo::class create A {}", "::A"),
            ("oo::configurable create B {}", "::B"),
            ("oo::abstract create C {}", "::C"),
            ("oo::singleton create D {}", "::D"),
            ("snit::type E {}", "::E"),
            ("snit::widget F {}", "::F"),
            ("snit::widgetadaptor G {}", "::G"),
            ("itcl::class H { variable x }", "::H"),
        ] {
            let mut ctx = registry_ctx();
            scan(src, None, "", false, &HashSet::new(), &mut ctx);
            assert!(
                ctx.result.classes.contains_key(key),
                "{src} should record class {key}"
            );
        }
    }

    #[test]
    fn qualified_definer_spellings_resolve_to_the_same_specs() {
        // The former name list carried a `::`-doubled variant of every
        // definer; the registry lookup's canonical leading-`::` fallback
        // covers them instead.
        for (src, key) in [
            ("::oo::class create A {}", "::A"),
            ("::oo::configurable create B {}", "::B"),
            ("::oo::abstract create C {}", "::C"),
            ("::oo::singleton create D {}", "::D"),
            ("::snit::type E {}", "::E"),
            ("::snit::widget F {}", "::F"),
            ("::snit::widgetadaptor G {}", "::G"),
            ("::itcl::class H { variable x }", "::H"),
        ] {
            let mut ctx = registry_ctx();
            scan(src, None, "", false, &HashSet::new(), &mut ctx);
            assert!(
                ctx.result.classes.contains_key(key),
                "{src} should record class {key}"
            );
        }
    }

    #[test]
    fn qualified_proc_spelling_recognised() {
        // `::proc` names the same global command as `proc`; canonical
        // registry resolution recognises it where the former name list
        // did not.
        let mut ctx = registry_ctx();
        scan(
            "::proc foo {} {}",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::foo"));
    }

    #[test]
    fn braced_body_non_definer_not_treated_as_definer() {
        // FP guard: a non-definer command with a braced trailing body must
        // record neither a class nor a proc.
        let mut ctx = registry_ctx();
        scan(
            "dict for {k v} $d { puts $k }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.classes.is_empty());
        assert!(ctx.result.procs.is_empty());
    }

    #[test]
    fn oo_define_extension_not_a_class_definer() {
        // FP guard: `oo::define` shares the TclOO definition-body grammar
        // but extends an existing class — the metaclass-trait gate must
        // keep it from minting a class named after its target.
        let mut ctx = registry_ctx();
        scan(
            "oo::define Shape { method area {} {} }",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.classes.is_empty());
    }

    #[test]
    fn oo_object_instance_creation_not_a_class_definer() {
        // FP guard: `oo::object` carries the metaclass trait but no
        // definition body — `oo::object create obj` makes an instance, not
        // a class.
        let mut ctx = registry_ctx();
        scan(
            "oo::object create obj",
            None,
            "",
            false,
            &HashSet::new(),
            &mut ctx,
        );
        assert!(ctx.result.classes.is_empty());
    }
}
