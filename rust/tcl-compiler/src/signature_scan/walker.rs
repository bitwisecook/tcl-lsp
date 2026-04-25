//! Top-level walker for the `signature_scan` module.
//!
//! Walks segmented commands and dispatches them to per-command
//! handlers in [`super::handlers`]. Body recursion into braced
//! scripts (proc bodies, namespace eval bodies, structured-command
//! branches) lives here too — it must not depend on the IR
//! lowering pass, which is the whole reason the `signature_scan`
//! module exists.
//!
//! Body recursion for `namespace eval`, `if`, `catch`, and `try`
//! is added incrementally in C40c2-C40c5 sub-strips; the dispatch
//! arms for those commands call no-op stubs in this scaffold strip.

#![allow(dead_code)]

use tcl_lexer::{Token, TokenType};

use super::ctx::ScanCtx;
use super::handlers;
use super::types::SignatureCommandInvocation;
use crate::segmenter::{segment_commands, segment_commands_with_offset};

/// Walk *source* as a Tcl script, emitting records for every command
/// the dispatcher recognises.
///
/// When `body_token` is `Some`, the spans on every record are
/// relocated into the outer source buffer's offset space (the body
/// token's content position is used as the base offset).
pub(super) fn scan(
    source: &str,
    body_token: Option<Token>,
    ns_prefix: &str,
    conditional: bool,
    ctx: &mut ScanCtx,
) {
    let commands = match body_token {
        None => segment_commands(source),
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
        ctx.result
            .command_invocations
            .push(SignatureCommandInvocation {
                name: head.to_string(),
                range: cmd.argv[0].span,
            });
        let texts = &cmd.texts;
        let argv = &cmd.argv;
        match head {
            "proc" => handlers::handle_proc(texts, argv, ns_prefix, ctx),
            "namespace" => handlers::handle_namespace(texts, argv, ns_prefix, conditional, ctx),
            "package" => handlers::handle_package(texts, argv, conditional, &mut ctx.result),
            "source" => handlers::handle_source(texts, argv, &mut ctx.result),
            "interp" => handlers::handle_interp(texts, &mut ctx.result),
            "oo::class" => handlers::handle_oo_class(texts, argv, ns_prefix, &mut ctx.result),
            "itcl::class" | "::itcl::class" => {
                handlers::handle_itcl_class(texts, argv, ns_prefix, &mut ctx.result);
            }
            "if" => handle_if_stub(texts, argv, ns_prefix, ctx),
            "catch" => handle_catch_stub(texts, argv, ns_prefix, ctx),
            "try" => handle_try_stub(texts, argv, ns_prefix, ctx),
            "lappend" | "set" => handlers::handle_auto_path(texts, argv, &mut ctx.result),
            _ => {
                handlers::maybe_handle_import_wrapper(
                    head,
                    texts,
                    argv,
                    ns_prefix,
                    &mut ctx.result,
                );
                handlers::maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx);
            }
        }
    }
}

/// Recurse into a braced body script.
///
/// Mirrors `_maybe_recurse_body` in
/// `core/analysis/signature_scan.py`. Only `Str` (braced) bodies
/// can be statically analysed; substituted bodies (`$body`,
/// `[gen_body]`) cannot be re-segmented and are skipped.
pub(super) fn maybe_recurse_body(
    body_text: &str,
    body_tok: Token,
    ns_prefix: &str,
    conditional: bool,
    ctx: &mut ScanCtx,
) {
    if body_tok.kind != TokenType::Str {
        return;
    }
    scan(body_text, Some(body_tok), ns_prefix, conditional, ctx);
}

// -- Body-recursion handler stubs ---------------------------------
// Filled in by C40c3 (`if`), C40c4 (`catch`), C40c5 (`try`).

fn handle_if_stub(_texts: &[String], _argv: &[Token], _ns_prefix: &str, _ctx: &mut ScanCtx) {
    // TODO(C40c3): walk every then/elseif/else branch via maybe_recurse_body.
}

fn handle_catch_stub(_texts: &[String], _argv: &[Token], _ns_prefix: &str, _ctx: &mut ScanCtx) {
    // TODO(C40c4): recurse into argv[1] body via maybe_recurse_body.
}

fn handle_try_stub(_texts: &[String], _argv: &[Token], _ns_prefix: &str, _ctx: &mut ScanCtx) {
    // TODO(C40c5): main body + on/trap handlers + finally.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_proc_emits_invocation_and_record() {
        let mut ctx = ScanCtx::default();
        scan("proc foo {} { set x 1 }", None, "", false, &mut ctx);
        assert!(ctx.result.procs.contains_key("::foo"));
        // command_invocations should contain at least the top-level "proc"
        // invocation; the body's "set" stays uninvoked since handle_proc
        // does not yet recurse (C40c7).
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
        let mut ctx = ScanCtx::default();
        scan(
            "package require Tcl 8.6\nsource /abs/path.tcl\nproc bar {} {}",
            None,
            "",
            false,
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
    fn namespace_eval_recurses_into_body() {
        let mut ctx = ScanCtx::default();
        scan(
            "namespace eval ns { proc inner {} {} }",
            None,
            "",
            false,
            &mut ctx,
        );
        assert!(ctx.result.procs.contains_key("::ns::inner"));
    }

    #[test]
    fn namespace_eval_absolute_rebases_prefix() {
        let mut ctx = ScanCtx::default();
        scan(
            "namespace eval outer { namespace eval ::abs { proc foo {} {} } }",
            None,
            "",
            false,
            &mut ctx,
        );
        // The inner ::abs eval should rebase, not nest under outer.
        assert!(ctx.result.procs.contains_key("::abs::foo"));
    }
}
