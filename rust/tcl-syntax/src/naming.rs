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

//! Variable and command name normalisation.
//!
//! These helpers live in the compiler-facing crates because they are
//! consumed by the expression parser and lowering — not by the lexer
//! itself.
//!
//! The `::`-qualifier split ([`qualifier_segments`] / [`is_qualified`]) is the
//! **one** canonical source for namespace-name parsing, shared by the compiler
//! (`normalise_qualified_name`) and the WASM runtime's command **and** variable
//! resolvers (`runtime/rust/src/namespace.rs`, the var coordinator) — mirroring
//! C Tcl's `TclGetNamespaceForQualName` segmentation (`tmp/tcl9.0.3`). Byte-based
//! so the runtime (which works in UTF-8 bytes) and the compiler (`&str`) share it
//! without one re-deriving the other.

/// Does `name` contain a `::` namespace separator (i.e. is it qualified)?
#[must_use]
pub fn is_qualified(name: &[u8]) -> bool {
    name.windows(2).any(|w| w == b"::")
}

/// Split a (possibly qualified) name on `::`, dropping empty segments — so
/// `::a::b::cmd` → `[a, b, cmd]`, `::cmd` → `[cmd]`, `cmd` → `[cmd]`, `::` → `[]`.
/// A run of **two or more** colons is one separator (all consecutive colons are
/// consumed), while a lone interior colon is an ordinary name character, so
/// `a:::b` → `[a, b]` and `a:b` → `[a:b]`. A trailing separator drops its empty
/// tail (`a::b::` → `[a, b]`); callers that care about the `{}`-named cmd/var a
/// trailing `::` denotes test for it themselves. Mirrors
/// `TclGetNamespaceForQualName`'s component walk (`tclNamesp.c`).
#[must_use]
pub fn qualifier_segments(name: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut seg_start = 0;
    let mut i = 0;
    while i < name.len() {
        if name[i] == b':' && i + 1 < name.len() && name[i + 1] == b':' {
            if i > seg_start {
                out.push(&name[seg_start..i]);
            }
            // C skips the `::` then every subsequent `:`, so a colon run of any
            // length is a single separator.
            i += 2;
            while i < name.len() && name[i] == b':' {
                i += 1;
            }
            seg_start = i;
        } else {
            i += 1;
        }
    }
    if seg_start < name.len() {
        out.push(&name[seg_start..]);
    }
    out
}

/// Does `name` end with a namespace separator (a run of ≥2 colons)? In a
/// command or variable name such a trailing `::` names the `{}` (empty) entity
/// in the qualified namespace (`TclGetNamespaceForQualName`), so the simple name
/// is `""` rather than the last [`qualifier_segments`] element.
#[must_use]
pub fn ends_with_separator(name: &[u8]) -> bool {
    name.len() >= 2 && name[name.len() - 1] == b':' && name[name.len() - 2] == b':'
}

/// Strip a variable reference's substitution sigil (`$`, `${…}`) while
/// **keeping** any array-index suffix — the form an evaluator needs to read the
/// actual variable (`$arr(idx)` → `arr(idx)`, `${v}` → `v`, `$x` → `x`). Unlike
/// [`normalise_var_name`], the `(idx)` is preserved.
///
/// ```
/// use tcl_syntax::naming::var_reference;
/// assert_eq!(var_reference("$arr(idx)"), "arr(idx)");
/// assert_eq!(var_reference("${v}"), "v");
/// assert_eq!(var_reference("$x"), "x");
/// ```
#[must_use]
pub fn var_reference(name: &str) -> &str {
    if let Some(inner) = name.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner
    } else if let Some(rest) = name.strip_prefix('$') {
        rest
    } else {
        name
    }
}

/// Normalise a Tcl variable reference to its base name.
///
/// Strips leading `$`, `${…}` delimiters, and array index `(…)`
/// suffixes:
///
/// ```
/// use tcl_syntax::naming::normalise_var_name;
/// assert_eq!(normalise_var_name("$foo"), "foo");
/// assert_eq!(normalise_var_name("${bar}"), "bar");
/// assert_eq!(normalise_var_name("$arr(idx)"), "arr");
/// assert_eq!(normalise_var_name("plain"), "plain");
/// ```
#[must_use]
pub fn normalise_var_name(name: &str) -> &str {
    let base = if let Some(inner) = name.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner
    } else if let Some(rest) = name.strip_prefix('$') {
        rest
    } else {
        name
    };

    // Strip array index: keep everything before the first `(`.
    match base.find('(') {
        Some(idx) => &base[..idx],
        None => base,
    }
}

/// Return `true` when `${name}` would successfully look up `name`.
///
/// Mirrors Tcl 9.0.3's `Tcl_ParseVarName` brace-form parser
/// (`tclParse.c` §1383+):
///
/// - `\X` (backslash + any char) consumes 2 source chars, both kept in
///   the lookup name — so a `}` preceded by `\` does not end the span.
/// - `{` / `}` are tracked with a depth counter; only a `}` at depth 0
///   ends the var-name span.
///
/// Returns `false` for a `}` at depth 0, a trailing lone `\`, or
/// unbalanced `{` (depth > 0 at end).  Drives the W215 reachability
/// check.
#[must_use]
pub fn is_brace_substitutable(name: &str) -> bool {
    if name.is_empty() {
        return true; // `${}` looks up the var literally named "".
    }
    let b = name.as_bytes();
    let n = b.len();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < n {
        match b[i] {
            b'}' if depth == 0 => return false,
            b'\\' => {
                if i + 1 >= n {
                    return false;
                }
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    depth == 0
}

/// Return `true` when `$name` would lex as a single bare variable token.
///
/// Mirrors `compiler/parsing/lexer._parse_var`'s bare-form rule: a name is
/// one or more `::`-separated segments, each consisting of Unicode alnum or
/// `_` characters, with an optional leading `::`.  Used to decide between
/// the bare `$name` and brace `${name}` forms in quick fixes.
///
/// ```
/// use tcl_syntax::naming::is_bare_var_name;
/// assert!(is_bare_var_name("foo"));
/// assert!(is_bare_var_name("ns::bar"));
/// assert!(is_bare_var_name("::baz"));
/// assert!(!is_bare_var_name("has-dash"));
/// assert!(!is_bare_var_name(""));
/// ```
#[must_use]
pub fn is_bare_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let s = name.strip_prefix("::").unwrap_or(name);
    if s.is_empty() {
        return false;
    }
    for segment in s.split("::") {
        if segment.is_empty() {
            return false;
        }
        if !segment.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
            return false;
        }
    }
    true
}

/// Return `true` when *word* is the braced indirect-array-element idiom
/// `${name}(index)`.
///
/// Tcl parses `${name}(index)` as the brace-form substitution `${name}`
/// (which the lexer ends at the `}`) concatenated with the *literal* text
/// `(index)`.  In a variable-name position (the target of `set` / `incr` /
/// `append` / `lappend` / `unset` / `info exists` / `vwait`) the resulting
/// string `<value-of-name>(index)` names element `index` of the array whose
/// name is held in the scalar `name` — the standard "array name kept in a
/// variable" idiom (e.g. `set ${token}(status) eof`).  The braces are
/// essential: the bare `$name(index)` is a *direct* array reference, a
/// different construct, so this returns `false` for it.
///
/// This is the discriminator that keeps the W216 (brace-then-paren) and W212
/// (substitution-where-name-expected) checks from false-positiving on the
/// indirect idiom.
///
/// ```
/// use tcl_syntax::naming::is_braced_indirect_array_ref;
/// assert!(is_braced_indirect_array_ref("${token}(status)"));
/// assert!(!is_braced_indirect_array_ref("$arr(idx)"));
/// assert!(!is_braced_indirect_array_ref("${x}"));
/// assert!(!is_braced_indirect_array_ref("${}(x)"));
/// ```
#[must_use]
pub fn is_braced_indirect_array_ref(word: &str) -> bool {
    if !word.starts_with("${") {
        return false;
    }
    let bytes = word.as_bytes();
    let n = bytes.len();
    // Walk to the depth-0 `}` that closes the brace-form variable name.
    let mut i = 2usize;
    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    while i < n {
        match bytes[i] {
            b'\\' if i + 1 < n => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    close = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    // No closing brace, or an empty name (`${}(…)`) — not the idiom.  The
    // closing `}` must sit past the `${` and the (non-empty) name.
    let Some(close) = close.filter(|&c| c > 2) else {
        return false;
    };
    // The closing `}` must be immediately followed by `(…)` running to the
    // end of the word.
    let rest = &bytes[close + 1..];
    rest.len() >= 2 && rest[0] == b'(' && *rest.last().unwrap() == b')'
}

/// Normalise a possibly-qualified Tcl command or procedure name.
///
/// Ensures the name starts with `::` and removes empty parts from
/// consecutive `::` separators.
///
/// ```
/// use tcl_syntax::naming::normalise_qualified_name;
/// assert_eq!(normalise_qualified_name("foo"), "::foo");
/// assert_eq!(normalise_qualified_name("ns::bar"), "::ns::bar");
/// assert_eq!(normalise_qualified_name("::baz"), "::baz");
/// assert_eq!(normalise_qualified_name(""), "");
/// assert_eq!(normalise_qualified_name("::::x"), "::x");
/// ```
#[must_use]
pub fn normalise_qualified_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Share the one canonical `::` segmentation. Each segment is a subslice of a
    // `&str` split on ASCII `::`, so it is valid UTF-8.
    let parts: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    if parts.is_empty() {
        return "::".to_owned();
    }
    format!("::{}", parts.join("::"))
}

/// Join a namespace `prefix` and a (possibly-relative) `name` into a fully
/// qualified, `::`-rooted name — the canonical Tcl qualification rule:
///
/// * An **absolute** `name` (leading `::`) ignores the prefix entirely —
///   `qualify("::ns", "::other::C")` is `::other::C`, never re-prefixed.
/// * A relative `name` resolves under `prefix`, which may be given rooted
///   (`::a::b`) or unrooted (`a::b`); duplicate separators collapse.
/// * An empty / root prefix roots the name at `::`.
///
/// The one shared join for the analyser / signature-scan / class-lattice
/// qualifiers, so the absolute-name rule cannot drift between them.
#[must_use]
pub fn qualify(prefix: &str, name: &str) -> String {
    if name.starts_with("::") {
        return normalise_qualified_name(name);
    }
    let p = prefix.trim_start_matches("::").trim_end_matches("::");
    if p.is_empty() {
        normalise_qualified_name(name)
    } else {
        normalise_qualified_name(&format!("{p}::{name}"))
    }
}

/// Candidate qualified names for Tcl's real bareword command/procedure
/// resolution, in priority order: the current namespace first, then global.
///
/// * An absolute name (`::foo`, `::ns::foo`) is taken as-is — one candidate.
/// * A relative name containing `::` (`inner::p`) is still resolved against
///   the current namespace first, **not** rooted straight at global: calling
///   `inner::p` from inside `namespace eval ::ns { … }` reaches
///   `::ns::inner::p` before `::inner::p`, when both exist (confirmed
///   against tclsh 9.0.4). Only a *leading* `::` is genuinely absolute.
/// * A bare name (`foo`) tries `{namespace}::foo` then `::foo` — exactly two
///   levels, never every enclosing ancestor namespace (Tcl's own command
///   lookup does not walk intermediate namespaces absent an explicit
///   `namespace path`, which this does not model).
///
/// Shared by every same-file "resolve this call the way Tcl would" consumer
/// — the analyser's same-file shadow/arity-suppression checks and the
/// optimiser's interprocedural proc-identity resolution — so a fix to the
/// resolution rule (or a bug in it) can't drift between them.
///
/// ```
/// use tcl_syntax::naming::bareword_resolution_candidates;
/// assert_eq!(bareword_resolution_candidates("::ns", "::foo"), vec!["::foo"]);
/// assert_eq!(
///     bareword_resolution_candidates("::ns", "inner::p"),
///     vec!["::ns::inner::p", "::inner::p"],
/// );
/// assert_eq!(bareword_resolution_candidates("::ns", "foo"), vec!["::ns::foo", "::foo"]);
/// assert_eq!(bareword_resolution_candidates("::", "foo"), vec!["::foo"]);
/// ```
#[must_use]
pub fn bareword_resolution_candidates(namespace: &str, cmd_name: &str) -> Vec<String> {
    if cmd_name.starts_with("::") {
        return vec![cmd_name.to_owned()];
    }
    let global = format!("::{cmd_name}");
    if namespace == "::" {
        return vec![global];
    }
    let relative = format!("{namespace}::{cmd_name}");
    vec![relative, global]
}

/// Split a Tcl variable reference into `(base, element)`.
///
/// Strips `$` / `${…}` substitution sigils first, then separates the
/// optional `(element)` array-index suffix from the base name.  Returns
/// `(base, None)` for scalar references.  Follows the brace-form rule that
/// `${arr}(foo)` is the scalar `arr` followed by literal `(foo)`, whereas
/// `${arr(foo)}` *is* the array element `arr(foo)`.
///
/// ```
/// use tcl_syntax::naming::split_array_name;
/// assert_eq!(split_array_name("arr"), ("arr", None));
/// assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
/// assert_eq!(split_array_name("${arr}"), ("arr", None));
/// ```
#[must_use]
pub fn split_array_name(name: &str) -> (&str, Option<&str>) {
    // `${…}` brace form: only the chars inside the braces are the reference;
    // an `(idx)` *inside* the braces is an element, one *after* `}` is not.
    if let Some(after) = name.strip_prefix("${")
        && let Some(rel) = after.find('}')
    {
        let inner = &after[..rel];
        if inner.ends_with(')')
            && let Some(idx) = inner.find('(')
        {
            return (&inner[..idx], Some(&inner[idx + 1..inner.len() - 1]));
        }
        return (inner, None);
    }
    // No closing brace — fall through (gated on `"}" in base`).
    let base = name.strip_prefix('$').unwrap_or(name);
    if base.ends_with(')')
        && let Some(idx) = base.find('(')
    {
        return (&base[..idx], Some(&base[idx + 1..base.len() - 1]));
    }
    (base, None)
}

/// True when `word` carries a variable / command substitution anywhere —
/// e.g. `rename ::$c mystr` or `rename foo bar[x]` — so it cannot be
/// resolved to a static command name at compile time.
///
/// ```
/// use tcl_syntax::naming::is_dynamic_word;
/// assert!(!is_dynamic_word("foo"));
/// assert!(is_dynamic_word("::$c"));
/// assert!(is_dynamic_word("bar[x]"));
/// ```
#[must_use]
pub fn is_dynamic_word(word: &str) -> bool {
    word.contains('$') || word.contains('[')
}

#[cfg(test)]
mod tests {

    #[test]
    fn qualify_joins_relative_names_under_rooted_and_unrooted_prefixes() {
        assert_eq!(qualify("::ns", "C"), "::ns::C");
        assert_eq!(qualify("ns", "C"), "::ns::C");
        assert_eq!(qualify("::a::b", "c::D"), "::a::b::c::D");
        assert_eq!(qualify("", "C"), "::C");
        assert_eq!(qualify("::", "C"), "::C");
    }

    #[test]
    fn qualify_keeps_absolute_names_absolute() {
        // The class-lattice regression: an absolute name must never be
        // re-prefixed under the current namespace.
        assert_eq!(qualify("::ns", "::other::C"), "::other::C");
        assert_eq!(qualify("ns", "::C"), "::C");
    }
    use super::*;

    #[test]
    fn simple_dollar() {
        assert_eq!(normalise_var_name("$foo"), "foo");
    }

    #[test]
    fn brace_substitutable_cases() {
        // Reachable names.
        assert!(is_brace_substitutable(""));
        assert!(is_brace_substitutable("foo"));
        assert!(is_brace_substitutable("a(b)")); // `)` is fine in brace form
        assert!(is_brace_substitutable("a{b}c")); // balanced inner braces
        assert!(is_brace_substitutable(r"a\}b")); // `\}` consumes 2, kept
        // Unreachable names.
        assert!(!is_brace_substitutable("a}b")); // `}` at depth 0 ends span early
        assert!(!is_brace_substitutable(r"trail\")); // trailing lone backslash
        assert!(!is_brace_substitutable("a{b")); // unbalanced `{`
    }

    #[test]
    fn split_array_name_forms() {
        assert_eq!(split_array_name("arr"), ("arr", None));
        assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
        assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
        assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
        // `${arr}(foo)` is scalar `arr` then literal `(foo)` — not an element.
        assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
        assert_eq!(split_array_name("${arr}"), ("arr", None));
        // dynamic index text is preserved verbatim for later classification.
        assert_eq!(split_array_name("a($i)"), ("a", Some("$i")));
        // a `)` with no `(` is not an element.
        assert_eq!(split_array_name("weird)"), ("weird)", None));
    }

    #[test]
    fn braced_dollar() {
        assert_eq!(normalise_var_name("${bar}"), "bar");
    }

    #[test]
    fn array_stripped() {
        assert_eq!(normalise_var_name("$arr(idx)"), "arr");
    }

    #[test]
    fn braced_array_stripped() {
        assert_eq!(normalise_var_name("${arr(idx)}"), "arr");
    }

    #[test]
    fn no_prefix() {
        assert_eq!(normalise_var_name("plain"), "plain");
    }

    #[test]
    fn namespace_qualified() {
        assert_eq!(normalise_var_name("$ns::var"), "ns::var");
    }

    #[test]
    fn empty_string() {
        assert_eq!(normalise_var_name(""), "");
    }

    #[test]
    fn bare_dollar() {
        assert_eq!(normalise_var_name("$"), "");
    }

    // normalise_qualified_name tests

    #[test]
    fn qualified_bare() {
        assert_eq!(normalise_qualified_name("foo"), "::foo");
    }

    #[test]
    fn qualified_already() {
        assert_eq!(normalise_qualified_name("::bar"), "::bar");
    }

    #[test]
    fn qualified_nested() {
        assert_eq!(normalise_qualified_name("ns::sub"), "::ns::sub");
    }

    #[test]
    fn qualified_empty() {
        assert_eq!(normalise_qualified_name(""), "");
    }

    #[test]
    fn qualified_just_colons() {
        assert_eq!(normalise_qualified_name("::"), "::");
    }

    #[test]
    fn qualified_extra_colons() {
        assert_eq!(normalise_qualified_name("::::x"), "::x");
    }

    // qualifier_segments / is_qualified

    #[test]
    fn qualifier_segments_cases() {
        assert_eq!(
            qualifier_segments(b"::a::b::cmd"),
            vec![&b"a"[..], b"b", b"cmd"]
        );
        assert_eq!(qualifier_segments(b"::cmd"), vec![&b"cmd"[..]]);
        assert_eq!(qualifier_segments(b"cmd"), vec![&b"cmd"[..]]);
        assert_eq!(qualifier_segments(b"a::b"), vec![&b"a"[..], b"b"]);
        assert!(qualifier_segments(b"::").is_empty());
        // a trailing separator drops the empty tail; a lone interior colon stays.
        assert_eq!(qualifier_segments(b"a::b::"), vec![&b"a"[..], b"b"]);
        // a run of >=2 colons is one separator (all consecutive colons consumed).
        assert_eq!(qualifier_segments(b"a:::b"), vec![&b"a"[..], b"b"]);
        assert_eq!(qualifier_segments(b"a::::b"), vec![&b"a"[..], b"b"]);
        assert_eq!(
            qualifier_segments(b":::test_ns_1:::::test_ns_2:::"),
            vec![&b"test_ns_1"[..], b"test_ns_2"]
        );
        // a lone interior colon is an ordinary name character.
        assert_eq!(qualifier_segments(b"a:b"), vec![&b"a:b"[..]]);
        assert!(ends_with_separator(b"a::b::"));
        assert!(!ends_with_separator(b"a::b"));
    }

    #[test]
    fn is_qualified_cases() {
        assert!(is_qualified(b"::a"));
        assert!(is_qualified(b"a::b"));
        assert!(is_qualified(b"::"));
        assert!(!is_qualified(b"plain"));
        assert!(!is_qualified(b"a:b"));
        assert!(!is_qualified(b""));
    }
}
