//! `namespace` text-op cores — the pure `::`-qualified-name manipulations
//! (`tail`, `qualifiers`), shared verbatim by both runtimes.
//!
//! The `tail`/`qualifiers` ops are pure byte→byte operations on the *literal*
//! name (no namespace resolution, no runtime state), so each runtime hands in
//! the name bytes and builds its own result value from the returned slice. The
//! `current`/`which` cores *do* read namespace state, so they are generic over
//! the [`Namespaces`] role trait + [`ValueOps`].

use tcl_runtime_api::Namespaces;
use tcl_syntax::value::ValueOps;

/// The byte range `(start, end)` of the last `::` separator **run** (two or more
/// consecutive colons) in `s`: `s[..start]` is the qualifier, `s[end..]` the
/// tail. `None` when there is no `::`. Mirrors C Tcl's `TclGetNamespaceForQualName`
/// colon-run handling — a run of 3+ colons is one separator (so `foo:::` has
/// qualifier `foo` and an empty tail), where a naive `rsplit("::")` diverges.
fn last_sep_run(s: &[u8]) -> Option<(usize, usize)> {
    // Scan back for the last "::" pair, then extend over every adjacent colon.
    let mut i = s.len();
    while i >= 2 {
        if s[i - 1] == b':' && s[i - 2] == b':' {
            let mut start = i - 2;
            while start > 0 && s[start - 1] == b':' {
                start -= 1;
            }
            let mut end = i;
            while end < s.len() && s[end] == b':' {
                end += 1;
            }
            return Some((start, end));
        }
        i -= 1;
    }
    None
}

/// `namespace qualifiers string` — everything before the last `::` run (the
/// empty string when `string` is unqualified).
#[must_use]
pub fn qualifiers(name: &[u8]) -> &[u8] {
    match last_sep_run(name) {
        Some((start, _)) => &name[..start],
        None => b"",
    }
}

/// `namespace tail string` — the simple name after the last `::` run (the whole
/// run is skipped, so `foo:::` yields the empty string).
#[must_use]
pub fn tail(name: &[u8]) -> &[u8] {
    match last_sep_run(name) {
        Some((_, end)) => &name[end..],
        None => name,
    }
}

/// `namespace current` — the fully-qualified name of the current namespace
/// (`"::"` at the global level).
pub fn current<O: ValueOps + Namespaces>(ops: &mut O) -> O::Value {
    let ns = Namespaces::current(ops);
    let name = ops.name(ns);
    ops.new_string(name)
}

/// `namespace which -command name` — the fully-qualified name `name` resolves to
/// as a command from the current namespace, or the empty string if it resolves
/// to nothing. (Option parsing and the `-variable` form stay in each adapter.)
pub fn which_command<O: ValueOps + Namespaces>(ops: &mut O, name: &str) -> O::Value {
    let cur = Namespaces::current(ops);
    match ops
        .find_command(cur, name)
        .and_then(|id| ops.command_name(id))
    {
        Some(fqn) => ops.new_string(fqn),
        None => ops.empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifiers_and_tail_match_c() {
        assert_eq!(qualifiers(b"::foo::bar"), b"::foo");
        assert_eq!(tail(b"::foo::bar"), b"bar");
        // Unqualified: no qualifier, the whole name is the tail.
        assert_eq!(qualifiers(b"bar"), b"");
        assert_eq!(tail(b"bar"), b"bar");
        // Colon runs (3+) are a single separator (C semantics; the naive
        // `rsplit("::")` the VM used yielded `:` for the tail here).
        assert_eq!(qualifiers(b"foo:::"), b"foo");
        assert_eq!(tail(b"foo:::"), b"");
        assert_eq!(qualifiers(b"a::::b"), b"a");
        assert_eq!(tail(b"a::::b"), b"b");
        // A trailing simple separator.
        assert_eq!(qualifiers(b"::foo::"), b"::foo");
        assert_eq!(tail(b"::foo::"), b"");
    }
}
