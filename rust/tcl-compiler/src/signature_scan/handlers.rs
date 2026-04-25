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

use super::types::{SignatureClass, SignatureScanResult};

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
}
