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

//! Constructed analytical keys: canonicalising a written command, and a key's tail, holder, segments and suffixes.

use super::written_names::{ends_with_separator, is_absolute, qualifier_segments};

/// Canonicalise a **written** command / variable word into the constructed-key
/// convention: a colon run of ≥2 is one separator (the whole run
/// collapses to `::`), a lone `:` is an ordinary name character, and a
/// *trailing* separator survives as `::` — it names the empty-string (`{}`)
/// command/variable in the qualified namespace, which is a real, addressable
/// entity (`proc x:: {} {}` defines it; `x::` / `x:::` / `::x::` all call it —
/// tclsh 8.6/9.0-verified, `TclGetNamespaceForQualName`, invariant 8.4→9.1).
///
/// An absolute input yields a rooted key (`::a:::b` → `::a::b`, `:::` → `::` —
/// the global `{}` command); a relative input yields a canonical *relative*
/// suffix (`a:::b` → `a::b`, `x::` → `x::`, `:` → `:`) for the caller to join
/// under a namespace key with one exact `::`.
///
/// This is for **written words only** — never re-apply it to a constructed
/// key: a key holding an all-colon *segment* (a namespace or command
/// legitimately named `:`) would collapse into its parent, which is exactly
/// how a `proc :` would collide with the empty-named `::` key.
///
/// ```
/// use tcl_syntax::naming::canonical_written_command;
/// assert_eq!(canonical_written_command("::a:::b"), "::a::b");
/// assert_eq!(canonical_written_command(":::"), "::");
/// assert_eq!(canonical_written_command("::x::"), "::x::");
/// assert_eq!(canonical_written_command("x:::"), "x::");
/// assert_eq!(canonical_written_command(":"), ":");
/// assert_eq!(canonical_written_command("a:b"), "a:b");
/// assert_eq!(canonical_written_command("cmd"), "cmd");
/// ```
#[must_use]
pub fn canonical_written_command(name: &str) -> String {
    let segs: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    let trailing = if ends_with_separator(name.as_bytes()) {
        "::"
    } else {
        ""
    };
    if is_absolute(name) {
        if segs.is_empty() {
            // Two-or-more colons alone: the global namespace's `{}` command.
            return "::".to_owned();
        }
        return format!("::{}{trailing}", segs.join("::"));
    }
    if segs.is_empty() {
        // A relative word cannot begin with a separator, so an empty segment
        // list means the word itself is empty.
        return String::new();
    }
    format!("{}{trailing}", segs.join("::"))
}

/// The simple tail selected by the authored constructed-key convention.
/// The input is an analytical presentation, independent of a written native
/// name or an actual namespace object's retained component path.
///
/// C's `namespace tail` consumes a whole colon run as one written separator.
/// This authored convention preserves a lone-colon simple name in examples
/// such as `":::"` and an empty simple name after `"::x::"`.
///
/// Joining arbitrary native components is not injective: `["a:", "b"]` and
/// `["a", ":b"]` both render as `"::a:::b"`. This helper cannot recover their
/// original holders or grant a native lookup identity. Native consumers retain
/// the exact component path and namespace incarnation independently.
///
/// ```
/// use tcl_syntax::naming::key_tail;
/// assert_eq!(key_tail("::a::b"), "b");
/// assert_eq!(key_tail(":::"), ":");            // "::" + ":"
/// assert_eq!(key_tail("::::::"), ":");         // ns `:` + proc `:`
/// assert_eq!(key_tail("::a:::"), ":");         // ns `a` + proc `:`
/// assert_eq!(key_tail("::x::"), "");           // the empty-named command
/// assert_eq!(key_tail("::"), "");              // global `{}` command key
/// assert_eq!(key_tail(":"), ":");              // bare simple name
/// assert_eq!(key_tail("a:b"), "a:b");
/// assert_eq!(key_tail("cmd"), "cmd");
/// ```
#[must_use]
pub fn key_tail(name: &str) -> &str {
    let bytes = name.as_bytes();
    if bytes.len() < 2 {
        return name;
    }
    if bytes.iter().all(|&b| b == b':') {
        // All-colon key.  The construction grammar — `"::"` root (2), plus 3
        // per `:`-named namespace level (`"::" + ":"`), plus a final `":"`
        // simple (1) or `""` simple (0) — makes length ≡ 0 (mod 3) the `:`
        // simple name and ≡ 2 (mod 3) the empty one; a lone `:` (length 1) is
        // the bare simple name itself, and other lengths are unconstructible.
        return match bytes.len() {
            n if n % 3 == 0 => &name[n - 1..],
            _ => "",
        };
    }
    // A constructed key's separator is a `::` whose suffix holds no further
    // `::` run and whose prefix is itself a valid namespace key.  Testing only
    // whether the prefix appears to end in a separator is insufficient:
    // `::a:::` is the valid key for namespace `:` below `::a`, despite its
    // final two bytes. Validate the construction grammar recursively (without
    // calling `key_tail`) so any depth of literal-colon segments is retained.
    let rooted = name.starts_with("::");
    let prefix_ok = |i: usize| {
        if rooted {
            i == 0 || (i > 2 && constructed_namespace_path_is_valid(&bytes[2..i]))
        } else {
            i > 0 && constructed_namespace_path_is_valid(&bytes[..i])
        }
    };
    let mut saw_empty_suffix = false;
    let mut i = bytes.len().saturating_sub(2);
    loop {
        if bytes[i] == b':' && bytes[i + 1] == b':' && prefix_ok(i) {
            let suffix = &name[i + 2..];
            if !suffix.contains("::") {
                if !suffix.is_empty() {
                    return suffix;
                }
                saw_empty_suffix = true;
            }
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    if saw_empty_suffix { "" } else { name }
}

/// Whether `path` is a non-empty sequence of constructed namespace segments.
/// Segments themselves contain no `::` run and are joined by exactly `::`;
/// they may contain a lone colon, including the whole segment `:`.  Colon runs
/// therefore have more than one possible split, so recognise the grammar with
/// a small position work-list instead of parsing a written Tcl name.
fn constructed_namespace_path_is_valid(path: &[u8]) -> bool {
    if path.is_empty() {
        return false;
    }
    let mut starts = vec![false; path.len() + 1];
    starts[0] = true;
    for start in 0..path.len() {
        if !starts[start] {
            continue;
        }
        for end in start + 1..=path.len() {
            if end >= start + 2 && path[end - 2] == b':' && path[end - 1] == b':' {
                break;
            }
            if end == path.len() {
                return true;
            }
            if end + 1 < path.len() && path[end] == b':' && path[end + 1] == b':' {
                starts[end + 2] = true;
            }
        }
    }
    false
}

/// Select an analytical holder and tail from a constructed display key.
/// This projection is not an inverse of native path construction: `["a:", "b"]`
/// and `["a", ":b"]` both render as `"::a:::b"`. Native lookup retains its
/// original component path and namespace incarnation instead. The holder of a root-level key is `"::"`
/// (`key_holder_and_tail("::x")` → `("::", "x")`), and the holder chain of a
/// colon-named nesting is preserved (`"::::::"` — proc `:` in the namespace
/// named `:` — → `(":::", ":")`).
///
/// ```
/// use tcl_syntax::naming::key_holder_and_tail;
/// assert_eq!(key_holder_and_tail("::a::b"), ("::a", "b"));
/// assert_eq!(key_holder_and_tail("::x"), ("::", "x"));
/// assert_eq!(key_holder_and_tail(":::"), ("::", ":"));
/// assert_eq!(key_holder_and_tail(":::::p"), (":::", "p"));
/// assert_eq!(key_holder_and_tail("::a:::::p"), ("::a:::", "p"));
/// assert_eq!(key_holder_and_tail("::::::"), (":::", ":"));
/// assert_eq!(key_holder_and_tail("::x::"), ("::x", ""));
/// assert_eq!(key_holder_and_tail("::"), ("::", ""));
/// ```
#[must_use]
pub fn key_holder_and_tail(key: &str) -> (&str, &str) {
    let tail = key_tail(key);
    if tail.len() + 2 > key.len() {
        // A bare simple name (no separator): the holder is unknown/relative.
        return ("", tail);
    }
    let holder = &key[..key.len() - tail.len() - 2];
    if holder.is_empty() {
        ("::", tail)
    } else {
        (holder, tail)
    }
}

/// Select analytical segments from a constructed namespace display key, one
/// [`key_holder_and_tail`] step per level. The selected split cannot recover
/// every native component path because `"::"`-joining is not injective.
/// A legitimately colon-named segment survives (`":::"` → `[":"]`,
/// `"::a::b"` → `["a", "b"]`, `"::"` → `[]`).  Accepts rooted or unrooted
/// keys.  Contrast [`qualifier_segments`], the *written-name* split, which
/// collapses colon runs.
///
/// ```
/// use tcl_syntax::naming::key_segments;
/// assert_eq!(key_segments("::a::b"), ["a", "b"]);
/// assert_eq!(key_segments("a::b"), ["a", "b"]);
/// assert_eq!(key_segments(":::"), [":"]);
/// assert_eq!(key_segments("::::::"), [":", ":"]);
/// assert!(key_segments("::").is_empty());
/// assert!(key_segments("").is_empty());
/// ```
#[must_use]
pub fn key_segments(key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor: String = if key.starts_with("::") || key.bytes().all(|b| b == b':') {
        key.to_owned()
    } else {
        format!("::{key}")
    };
    while !cursor.is_empty() && cursor != "::" {
        let (holder, tail) = key_holder_and_tail(&cursor);
        out.push(tail.to_owned());
        if holder == cursor {
            break;
        }
        cursor = holder.to_owned();
    }
    out.reverse();
    out
}

/// Possible suffix labels of a constructed analytical key, retaining the
/// complete input first. Each suffix follows the existing [`key_segments`]
/// projection; lone colon segments and a trailing empty segment survive.
/// Empty suffixes are omitted and repeated labels appear once.
///
/// These labels are a conservative metadata index, not written Tcl spellings,
/// namespace paths, command lookup candidates or proof of a selected binding.
#[must_use]
pub fn key_suffixes(key: &str) -> Vec<String> {
    let mut suffixes = vec![key.to_owned()];
    let segments = key_segments(key);
    for start in 0..segments.len() {
        let suffix = segments[start..].join("::");
        if !suffix.is_empty() && !suffixes.contains(&suffix) {
            suffixes.push(suffix);
        }
    }
    suffixes
}

/// Whether a definition written as `simple_name` inside the namespace whose
/// **written segments** are `ns_segments` can be reached by *any* absolute
/// (fully-qualified) written form.
///
/// Unaddressable shapes (tclsh 8.6/9.0-verified, invariant in C 8.4→9.1):
/// an all-colon simple name (only `:` is writable — a written colon run of ≥2
/// is a separator, so no absolute spelling can end in that name), or any
/// namespace segment that is itself all-colons (`namespace eval :` — its
/// contents are reachable only relatively, e.g. `namespace inscope : :`; the
/// rendered `[namespace which]` form `::::::` does not resolve).  The
/// empty-string name (`proc {} {} {}` / `proc x:: {} {}`) **is** addressable
/// (`::` / `::x::`).
#[must_use]
pub fn is_absolutely_addressable(ns_segments: &[&str], simple_name: &str) -> bool {
    let all_colons = |s: &str| !s.is_empty() && s.bytes().all(|b| b == b':');
    !all_colons(simple_name) && !ns_segments.iter().any(|seg| all_colons(seg))
}
