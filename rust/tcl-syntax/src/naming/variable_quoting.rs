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

//! Which source spellings can carry a variable name: bare `$name`, `${name}`, names that need quoting and the braced indirect-array idiom.

/// Whether a variable's **name** can only ever appear in source inside
/// quoting — `{$n}` / `${$n}`, `{a b}` / `${a b}` — because writing it bare
/// would substitute, split the word, or end the command.
///
/// Such a name is legal and distinct in Tcl (`set {$n} v` creates the
/// variable `$n`, unrelated to `n`), but every occurrence of it carries
/// delimiters that are *not* part of the recorded name span, and the
/// delimiters a **new** name needs depend on that new name.  A rewrite is
/// therefore not a matter of substituting new text into the recorded spans,
/// which is what the rename edit builder does — so rename refuses instead of
/// emitting an edit set that would produce `set q} 1`.
///
/// A namespace-qualified bareword (`::ns::v`) needs no quoting; neither does
/// an array element whose base and key are barewords (`arr(k)`), since it is
/// keyed by its base.
///
/// ```
/// use tcl_syntax::naming::var_name_requires_quoting;
/// assert!(var_name_requires_quoting("$n"));
/// assert!(var_name_requires_quoting("a b"));
/// assert!(var_name_requires_quoting("[gen]"));
/// assert!(!var_name_requires_quoting("n"));
/// assert!(!var_name_requires_quoting("::ns::v"));
/// assert!(!var_name_requires_quoting("arr"));
/// ```
#[must_use]
pub fn var_name_requires_quoting(name: &str) -> bool {
    name.is_empty()
        || name.contains([
            '$', '[', ']', '{', '}', '"', '\\', ';', ' ', '\t', '\n', '\r',
        ])
}

/// Return `true` when `${name}` would successfully look up `name` under the
/// given `${…}` delimiting rule.
///
/// `nesting` selects the dialect's `Tcl_ParseVarName` brace-form parser
/// (`LexerConfig::braced_var`):
///
/// - **`true` — Tcl 9.x** (tcl9.0.1 `tclParse.c`, the `braceCount` loop):
///   `\X` consumes 2 source chars, both kept in the lookup name — so a `}`
///   preceded by `\` does not end the span — and `{` / `}` are tracked with
///   a depth counter; only a `}` at depth 0 ends the var-name span.
///   Unreachable: a `}` at depth 0, a trailing lone `\`, or unbalanced `{`.
/// - **`false` — the 8.x family** (8.6.14 `tclParse.c:1466`,
///   tclsh-verified): the name is everything up to the FIRST literal `}`,
///   with no nesting and no backslash processing — so any name containing a
///   `}` is unreachable, and `{` / `\` are ordinary name characters.
///
/// Drives the W215 reachability check.
#[must_use]
pub fn is_brace_substitutable(name: &str, nesting: bool) -> bool {
    if name.is_empty() {
        return true; // `${}` looks up the var literally named "".
    }
    if !nesting {
        // 8.x: the first literal `}` closes the form, so a name containing
        // one can never be spelt; everything else is verbatim.
        return !name.contains('}');
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
/// A name is one or more `::`-separated segments of **ASCII** alphanumerics
/// or `_`, with an optional leading `::` — `TclIsBareword` is ASCII-only in
/// every Tcl (8.6.14 and 9.0.1 `tclParse.c`; the man page: "Letters and
/// digits are only the standard ASCII ones"), so `$café` names the variable
/// `caf`. Used to decide between the bare `$name` and brace `${name}` forms
/// in quick fixes — a Unicode-permissive rule here would let a
/// `${café}` → `$café` rewrite silently change which variable is read.
///
/// ```
/// use tcl_syntax::naming::is_bare_var_name;
/// assert!(is_bare_var_name("foo"));
/// assert!(is_bare_var_name("ns::bar"));
/// assert!(is_bare_var_name("::baz"));
/// assert!(!is_bare_var_name("has-dash"));
/// assert!(!is_bare_var_name("café"));
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
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
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
