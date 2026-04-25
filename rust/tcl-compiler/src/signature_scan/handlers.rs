//! Per-command handlers used by the `signature_scan` walker.
//!
//! Each handler is a small free function that takes the segmented
//! command pieces (head text, per-arg texts, per-arg representative
//! tokens, surrounding namespace prefix) and mutates a [`ScanCtx`].
//! Handlers do not recurse into bodies themselves — body recursion
//! lives in [`super::walker`] and is wired in when the walker sub-strips
//! land.
//!
//! [`ScanCtx`]: super::ctx::ScanCtx

#![allow(dead_code)]

use tcl_lexer::Token;

use super::ctx::{ProcBodyInfo, ScanCtx};
use super::params::parse_param_list;
use super::types::{SignatureClass, SignatureProc, SignatureScanResult};

/// Fully qualify `name` within `ns_prefix` following Tcl scoping.
///
/// Mirrors `_qualify` in `core/analysis/signature_scan.py`. Absolute
/// names (those starting with `::`) ignore the prefix entirely so a
/// `proc ::foo::bar` declared inside `namespace eval baz` still
/// indexes as `::foo::bar`.
///
/// `ns_prefix` is expected to be the call-site namespace **without**
/// a leading `::` (the walker carries it that way to match the
/// Python convention).
pub(super) fn qualify(ns_prefix: &str, name: &str) -> String {
    if name.starts_with("::") {
        name.to_string()
    } else if ns_prefix.is_empty() {
        format!("::{name}")
    } else {
        format!("::{ns_prefix}::{name}")
    }
}

/// Insert a class record under `result.classes`, computing the
/// qualified name + simple-name split.
///
/// Mirrors `_emit_class` in `core/analysis/signature_scan.py`.
/// Shared by `handle_oo_class` and `handle_itcl_class` so both
/// `oo::class create NAME ?BODY?` and `itcl::class NAME BODY`
/// produce identically-shaped records.
pub(super) fn emit_class(
    raw_name: &str,
    name_tok: Token,
    body_tok: Token,
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    let qualified = qualify(ns_prefix, raw_name);
    let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
    result.classes.insert(
        qualified.clone(),
        SignatureClass {
            name: simple,
            qualified_name: qualified,
            name_range: name_tok.span,
            body_range: body_tok.span,
        },
    );
}

/// Handler for `proc NAME PARAMS BODY`.
///
/// Mirrors `_handle_proc` in `core/analysis/signature_scan.py`.
/// Records a `SignatureProc` in `ctx.result.procs` and a
/// `ProcBodyInfo` in `ctx.proc_bodies` so the second-pass factory
/// resolver can identify factory-wrapper procs by their
/// `proc $a $b $c` body shape.
///
/// Body recursion (`scan_factory_candidates`) is **not** wired in
/// this strip — it lands in `C40c7`, after the walker module
/// exists. The proc is recorded; the body walk is deferred.
pub(super) fn handle_proc(texts: &[String], argv: &[Token], ns_prefix: &str, ctx: &mut ScanCtx) {
    if texts.len() < 4 {
        return;
    }
    let raw_name = &texts[1];
    let qualified = qualify(ns_prefix, raw_name);
    let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
    let name_range = argv[1].span;
    let body_range = argv[3].span;
    let params = parse_param_list(&texts[2]);
    let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
    ctx.result.procs.insert(
        qualified.clone(),
        SignatureProc {
            name: simple,
            qualified_name: qualified.clone(),
            params,
            name_range,
            body_range,
        },
    );
    let body_ns = match qualified.rsplit_once("::") {
        Some((parent, _)) => parent.trim_start_matches(':').to_string(),
        None => String::new(),
    };
    ctx.proc_bodies.push(ProcBodyInfo {
        qname: qualified,
        params: param_names,
        body_text: texts[3].clone(),
        ns_prefix: body_ns,
    });
    // TODO(C40c7): scan factory candidates in body when argv[3] is `Str`.
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{Span, TokenType};

    fn token(start: u32, end: u32) -> Token {
        Token::new(TokenType::Esc, Span::new(start, end))
    }

    #[test]
    fn absolute_name_ignores_prefix() {
        assert_eq!(qualify("foo::bar", "::baz"), "::baz");
        assert_eq!(qualify("", "::top"), "::top");
        assert_eq!(qualify("nested", "::ns::deep"), "::ns::deep");
    }

    #[test]
    fn relative_name_under_nested_ns() {
        assert_eq!(qualify("foo", "bar"), "::foo::bar");
        assert_eq!(qualify("foo::baz", "qux"), "::foo::baz::qux");
    }

    #[test]
    fn empty_prefix_promotes_to_global() {
        assert_eq!(qualify("", "bar"), "::bar");
    }

    #[test]
    fn emit_class_under_namespace_indexes_bare_name() {
        let mut result = SignatureScanResult::default();
        emit_class("MyCls", token(10, 15), token(20, 30), "ns", &mut result);
        let cls = result.classes.get("::ns::MyCls").expect("inserted");
        assert_eq!(cls.name, "MyCls");
        assert_eq!(cls.qualified_name, "::ns::MyCls");
        assert_eq!(cls.name_range, Span::new(10, 15));
        assert_eq!(cls.body_range, Span::new(20, 30));
    }

    #[test]
    fn emit_class_with_absolute_name_preserves_indexing() {
        let mut result = SignatureScanResult::default();
        emit_class("::Top", token(0, 5), token(6, 8), "ns", &mut result);
        let cls = result
            .classes
            .get("::Top")
            .expect("absolute name preserved");
        assert_eq!(cls.name, "Top");
        assert_eq!(cls.qualified_name, "::Top");
    }

    fn proc_inputs(name: &str) -> (Vec<String>, Vec<Token>) {
        let texts = vec![
            "proc".to_string(),
            name.to_string(),
            "a b".to_string(),
            "set x 1".to_string(),
        ];
        let argv = vec![token(0, 4), token(5, 8), token(10, 13), token(15, 22)];
        (texts, argv)
    }

    #[test]
    fn handle_proc_records_bare_proc() {
        let (texts, argv) = proc_inputs("foo");
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "", &mut ctx);
        let proc = ctx.result.procs.get("::foo").expect("inserted");
        assert_eq!(proc.name, "foo");
        assert_eq!(proc.qualified_name, "::foo");
        assert_eq!(proc.params.len(), 2);
        assert_eq!(proc.params[0].name, "a");
        assert_eq!(proc.name_range, Span::new(5, 8));
        assert_eq!(proc.body_range, Span::new(15, 22));
        assert_eq!(ctx.proc_bodies.len(), 1);
        assert_eq!(ctx.proc_bodies[0].qname, "::foo");
        assert_eq!(ctx.proc_bodies[0].body_text, "set x 1");
        assert_eq!(ctx.proc_bodies[0].ns_prefix, "");
    }

    #[test]
    fn handle_proc_under_namespace_indexes_qualified() {
        let (texts, argv) = proc_inputs("bar");
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "ns::deep", &mut ctx);
        let proc = ctx.result.procs.get("::ns::deep::bar").expect("inserted");
        assert_eq!(proc.name, "bar");
        assert_eq!(ctx.proc_bodies[0].ns_prefix, "ns::deep");
    }

    #[test]
    fn handle_proc_with_absolute_name_records_at_global() {
        let (texts, argv) = proc_inputs("::top::baz");
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "outer", &mut ctx);
        let proc = ctx.result.procs.get("::top::baz").expect("inserted");
        assert_eq!(proc.name, "baz");
        assert_eq!(proc.qualified_name, "::top::baz");
        // body_ns drops the leading colon and trailing simple name.
        assert_eq!(ctx.proc_bodies[0].ns_prefix, "top");
    }

    #[test]
    fn handle_proc_too_few_args_no_op() {
        let texts = vec!["proc".to_string(), "name".to_string()];
        let argv = vec![token(0, 4), token(5, 9)];
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "", &mut ctx);
        assert!(ctx.result.procs.is_empty());
        assert!(ctx.proc_bodies.is_empty());
    }
}
