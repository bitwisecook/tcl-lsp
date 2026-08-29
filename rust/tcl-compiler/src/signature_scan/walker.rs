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
//!   `handle_if` / `handle_catch` / `handle_try` here.
//! - [`scan_factory_candidates`] — secondary walker called from
//!   `handle_proc`; only collects four-token factory candidates and
//!   recurses into structural-control bodies via
//!   `scan_factory_structural`.

use std::collections::HashSet;

use tcl_lexer::{Token, TokenType};
use tcl_registry::Traits;
use tcl_registry::definer::DefinerFamily;
use tcl_registry::hooks::{AnalyserHookId, LoweringHookId};
use tcl_registry::{CommandSpec, SubCommand};

use super::command_prefix::command_prefix_invocations;
use super::ctx::ScanCtx;
use super::handlers;
use super::types::SignatureCommandInvocation;
use tcl_dialect::model::Family;
use crate::segmenter::{
    SegmentedCommand, segment_commands_with_offset, segment_commands_with_recovery,
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
pub(super) fn scan(
    source: &str,
    body_token: Option<Token>,
    ns_prefix: &str,
    conditional: bool,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    let commands = match body_token {
        None => segment_commands_with_recovery(source, known_commands),
        Some(tok) => {
            let base = tok.span.start() + u32::from(tok.content_offset);
            segment_commands_with_offset(source, base)
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
                name: head.to_string(),
                range: cmd.argv[0].span,
                // Signature scan skips the scope walk; leave
                // the resolved name unpopulated and let the
                // full analyser fill it in when the same
                // document is reopened in the foreground.
                resolved_qualified_name: None,
                resolved_user_definition: false,
                resolution_candidates: Vec::new(),
                argc: arg_count,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: false,
                is_mathfunc_call: false,
                ensemble_dispatch: None,
            });
        // Record command-prefix callback heads (`lsort -command cb`, `trace
        // add … cb`, …) as their own invocations so background-scanned files
        // feed find-references / call-hierarchy / usage counts / callback
        // arity through the same substrate as ordinary calls.
        record_command_prefix_invocations(&cmd, head, ctx);
        let texts = &cmd.texts;
        let argv = &cmd.argv;
        let handled = resolve_scan_dispatch(ctx.registry, head, texts).is_some_and(|dispatch| {
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
            handlers::maybe_handle_import_wrapper(head, texts, argv, ns_prefix, &mut ctx.result);
            handlers::maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx);
        }
    }
}

#[derive(Clone, Copy)]
struct ResolvedScanDispatch<'r> {
    spec: &'r CommandSpec,
    subcommand: Option<&'r SubCommand>,
    analyser: Option<AnalyserHookId>,
    lowering: Option<LoweringHookId>,
}

fn resolve_scan_dispatch<'r>(
    registry: Option<&'r tcl_registry::CommandRegistry>,
    head: &str,
    texts: &[String],
) -> Option<ResolvedScanDispatch<'r>> {
    let spec = registry?.get(head)?;
    let subcommand = texts.get(1).and_then(|word| spec.resolve_subcommand(word));
    Some(ResolvedScanDispatch {
        spec,
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
        Some(AnalyserHookId::NamespaceImport) => handlers::handle_namespace_import(
            texts,
            argv,
            ns_prefix,
            dispatch.subcommand,
            &mut ctx.result,
        ),
        Some(AnalyserHookId::NamespaceForget) => handlers::handle_namespace_forget(
            texts,
            argv,
            ns_prefix,
            dispatch.subcommand,
            &mut ctx.result,
        ),
        Some(AnalyserHookId::PackageRequire) => handlers::handle_package_require(
            texts,
            argv,
            conditional,
            dispatch.subcommand,
            &mut ctx.result,
        ),
        Some(AnalyserHookId::Source) => {
            handlers::handle_source(texts, argv, ns_prefix, dispatch.spec, &mut ctx.result);
        }
        Some(AnalyserHookId::InterpAlias) => {
            handlers::handle_interp_alias(texts, &mut ctx.result);
        }
        Some(AnalyserHookId::Rename) => {
            handlers::handle_rename(texts, ns_prefix, &mut ctx.result);
        }
        Some(AnalyserHookId::Catch) => {
            handle_catch(texts, argv, ns_prefix, known_commands, ctx);
        }
        Some(AnalyserHookId::Try) => {
            handle_try(texts, argv, ns_prefix, known_commands, ctx);
        }
        Some(AnalyserHookId::Set | AnalyserHookId::Lappend) => {
            handlers::handle_auto_path(texts, argv, &mut ctx.result);
        }
        _ if dispatch.lowering == Some(LoweringHookId::If) => {
            handle_if(texts, argv, ns_prefix, known_commands, ctx);
        }
        _ => return false,
    }
    true
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
        return match grammar.family {
            // Every stock `TclOO` metaclass creates a class via the same
            // `METACLASS create NAME ?BODY?` interface — `oo::configurable`
            // (property-bearing), `oo::abstract`, and `oo::singleton`
            // included, so a `[Pin new]` on an `oo::configurable` class is
            // typed as an object like any other (issue #797).
            DefinerFamily::TclOo if spec.traits.contains(Traits::IS_OO_METACLASS) => {
                if let Some(method) = texts
                    .get(1)
                    .and_then(|word| ctx.registry?.exported_manufacturer_method(head, word))
                {
                    handlers::handle_oo_class(texts, argv, method, ns_prefix, &mut ctx.result);
                }
                true
            }
            // `oo::define` / `oo::objdefine` share the `TclOO` grammar but
            // extend an existing class rather than create one — and a
            // `.tclspec` declaration body manufactures nothing at all: it
            // describes commands rather than creating them. Neither records a
            // definition here; the ordinary scan continues past both.
            DefinerFamily::TclOo | DefinerFamily::SpecTcl => false,
            // snit types/widgets create instances via `Name create obj` /
            // `Name %AUTO%` / a widget's `Name .path`, so record them as
            // classes to type those constructors' receivers (same shape as
            // itcl).
            DefinerFamily::Snit => {
                handlers::handle_snit_type(texts, argv, ns_prefix, &mut ctx.result);
                true
            }
            DefinerFamily::Itcl => {
                handlers::handle_itcl_class(texts, argv, ns_prefix, &mut ctx.result);
                true
            }
        };
    }
    // `tcl::OptProc name optlist body` (issue #923 idx 90): a real proc
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
fn record_command_prefix_invocations(cmd: &SegmentedCommand, head: &str, ctx: &mut ScanCtx<'_>) {
    let Some(registry) = ctx.registry else {
        return;
    };
    if cmd.texts.len() < 2 || cmd.argv.len() < 2 || cmd.single_token_word.len() < 2 {
        return;
    }
    let invs = command_prefix_invocations(
        registry,
        head,
        super::command_prefix::CommandPrefixWords {
            texts: &cmd.texts[1..],
            tokens: &cmd.argv[1..],
            single_token: &cmd.single_token_word[1..],
            expanded: cmd
                .expand_word
                .as_deref()
                .and_then(|expanded| expanded.get(1..))
                .unwrap_or(&[]),
            source_map: None,
        },
    );
    for inv in invs {
        ctx.result
            .command_invocations
            .push(SignatureCommandInvocation {
                name: inv.head,
                range: inv.span,
                // Signature scan skips scope resolution (walker contract).
                resolved_qualified_name: None,
                resolved_user_definition: false,
                resolution_candidates: Vec::new(),
                // The legacy direct-call arity path always skips a callback
                // head (`None`); the callback-arity check reads
                // `callback_baked_args` + `callback_arity`.
                argc: None,
                callback_arity: Some(inv.appended),
                callback_baked_args: inv.baked,
                indirect: false,
                rename_safe: true,
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

fn handle_if(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    // Tcl's `if` takes the shape:
    //   if EXPR ?then? BODY ?elseif EXPR ?then? BODY?... ?else? ?BODY?
    // Alternate between expecting an expression and expecting a body,
    // resetting the expectation whenever `then` / `elseif` / `else`
    // appears. Every recursed body is marked `conditional=true`.
    let mut i = 1;
    let mut expect_body = false;
    while i < texts.len() {
        let word = texts[i].as_str();
        if word == "then" {
            expect_body = true;
            i += 1;
            continue;
        }
        if word == "elseif" {
            expect_body = false;
            i += 1;
            continue;
        }
        if word == "else" {
            expect_body = true;
            i += 1;
            continue;
        }
        if expect_body {
            maybe_recurse_body(&texts[i], argv[i], ns_prefix, true, known_commands, ctx);
            expect_body = false;
        } else {
            expect_body = true;
        }
        i += 1;
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

fn handle_try(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    known_commands: &HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    // `try BODY ?on CODE VARLIST BODY?... ?trap PATTERN VARLIST BODY?...
    //  ?finally BODY?` — the main body sits at index 1; handler clauses
    // (`on`/`trap`) take 4 words each with the body at +3; `finally`
    // takes 2 words with the body at +1.
    if texts.len() < 2 {
        return;
    }
    maybe_recurse_body(&texts[1], argv[1], ns_prefix, true, known_commands, ctx);
    let mut i = 2;
    while i < texts.len() {
        let clause = texts[i].as_str();
        if clause == "finally" && i + 1 < texts.len() {
            maybe_recurse_body(
                &texts[i + 1],
                argv[i + 1],
                ns_prefix,
                true,
                known_commands,
                ctx,
            );
            return;
        }
        if (clause == "on" || clause == "trap") && i + 3 < texts.len() {
            maybe_recurse_body(
                &texts[i + 3],
                argv[i + 3],
                ns_prefix,
                true,
                known_commands,
                ctx,
            );
            i += 4;
        } else {
            i += 1;
        }
    }
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
pub(super) fn scan_factory_candidates(
    body_text: &str,
    body_tok: Token,
    ns_prefix: &str,
    ctx: &mut ScanCtx,
) {
    let base = body_tok.span.start() + u32::from(body_tok.content_offset);
    let commands = segment_commands_with_offset(body_text, base);
    for cmd in commands {
        if cmd.is_partial || cmd.argv.is_empty() {
            continue;
        }
        let head = cmd.name();
        if head.is_empty() {
            continue;
        }
        let texts = &cmd.texts;
        let argv = &cmd.argv;
        let structural = resolve_scan_dispatch(ctx.registry, head, texts)
            .is_some_and(|dispatch| scan_factory_structural(dispatch, texts, argv, ns_prefix, ctx));
        if !structural {
            handlers::maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx);
        }
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
        let inner = if let Some(rest) = raw_ns.strip_prefix("::") {
            rest.trim_start_matches(':').to_string()
        } else if !ns_prefix.is_empty() {
            format!("{ns_prefix}::{raw_ns}")
        } else {
            raw_ns.clone()
        };
        if argv[3].kind == TokenType::Str {
            scan_factory_candidates(&texts[3], argv[3], &inner, ctx);
        }
        return true;
    }
    if dispatch.lowering == Some(LoweringHookId::If) {
        let mut i = 1;
        let mut expect_body = false;
        while i < texts.len() {
            let w = texts[i].as_str();
            if w == "then" {
                expect_body = true;
                i += 1;
                continue;
            }
            if w == "elseif" {
                expect_body = false;
                i += 1;
                continue;
            }
            if w == "else" {
                expect_body = true;
                i += 1;
                continue;
            }
            if expect_body && argv[i].kind == TokenType::Str {
                scan_factory_candidates(&texts[i], argv[i], ns_prefix, ctx);
                expect_body = false;
            } else {
                expect_body = true;
            }
            i += 1;
        }
        return true;
    }
    if dispatch.analyser == Some(AnalyserHookId::Catch) {
        if texts.len() >= 2 && argv[1].kind == TokenType::Str {
            scan_factory_candidates(&texts[1], argv[1], ns_prefix, ctx);
        }
        return true;
    }
    if dispatch.analyser == Some(AnalyserHookId::Try) && texts.len() >= 2 {
        if argv[1].kind == TokenType::Str {
            scan_factory_candidates(&texts[1], argv[1], ns_prefix, ctx);
        }
        let mut i = 2;
        while i < texts.len() {
            let clause = texts[i].as_str();
            if clause == "finally" && i + 1 < texts.len() && argv[i + 1].kind == TokenType::Str {
                scan_factory_candidates(&texts[i + 1], argv[i + 1], ns_prefix, ctx);
                return true;
            }
            if (clause == "on" || clause == "trap")
                && i + 3 < texts.len()
                && argv[i + 3].kind == TokenType::Str
            {
                scan_factory_candidates(&texts[i + 3], argv[i + 3], ns_prefix, ctx);
                i += 4;
            } else {
                i += 1;
            }
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
        // TP guard: every previously name-listed class definer still emits a
        // class record via registry dispatch.
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
