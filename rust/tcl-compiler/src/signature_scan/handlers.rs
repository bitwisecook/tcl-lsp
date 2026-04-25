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

#[cfg(test)]
mod tests {
    use super::*;

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
}
