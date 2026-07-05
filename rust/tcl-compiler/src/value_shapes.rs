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

//! Shared Tcl value-shape helpers used across compiler passes.

/// Scan one Tcl variable reference starting at `text[at]`, returning the
/// byte index just past its end, or `None` when `text[at..]` does not
/// start with a valid reference.
///
/// Handles the three forms from the Tcl 9.0 `Tcl_ParseVar` spec:
///
/// - `$name` — bare name in `[A-Za-z0-9_:]`; `::` runs stay part of the
///   name (namespace-qualified).
/// - `${name}` — braced; any character except `}` (escapes inside the
///   braces are taken literally).
/// - `$name(index)` — array element. The index runs to the matching `)`,
///   with backslash-escaped close parens taken literally (so `$a(x\)y)`
///   is one reference whose index text is `x\)y`).
fn scan_pure_var_ref(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(at) != Some(&b'$') {
        return None;
    }
    let mut i = at + 1;
    // Braced form `${...}`.
    if bytes.get(i) == Some(&b'{') {
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b'}' {
            j += 1;
        }
        if j >= bytes.len() {
            return None;
        }
        return Some(j + 1);
    }
    // Bare name: alphanumeric + `_` + `:`.
    let name_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b':')) {
        i += 1;
    }
    if i == name_start {
        return None;
    }
    // Optional array index `(index)` — backslash-escape aware.
    if bytes.get(i) == Some(&b'(') {
        let mut j = i + 1;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' if j + 1 < bytes.len() => j += 2, // skip escaped char
                b')' => return Some(j + 1),
                _ => j += 1,
            }
        }
        return None; // unterminated index
    }
    Some(i)
}

/// Return `true` when `text` is exactly one variable reference (`$x` /
/// `${x}` / `$ns::x` / `$arr(idx)`) with no surrounding or concatenated
/// syntax.
///
/// The check gates the uplevel/safe-eval idioms: `uplevel 1 $body` is
/// single-substitution-safe only when `$body` is one pure reference;
/// anything else double-substitutes the composed value.
#[must_use]
pub fn is_pure_var_ref(text: &str) -> bool {
    scan_pure_var_ref(text, 0) == Some(text.len())
}

/// Return `true` when `text` is a braced array-shaped var ref `${a(1)}`.
///
/// tclsh 9 loads such a reference by its whole literal name (`a(1)`
/// resolved at runtime as array element `a(1)`), so the value is a
/// *variable reference*, never a constant — analyses must treat it as
/// conservatively unknown (overdefined), the same as `${ns::y}`. A plain
/// braced scalar `${foo}` (no parens) is *not* matched.
#[must_use]
pub fn is_braced_whole_name_array_ref(text: &str) -> bool {
    let Some(inner) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) else {
        return false;
    };
    inner.contains('(') && inner.ends_with(')') && !inner.contains('}')
}

/// Extract command name and args from `[cmd arg1 arg2 …]`.
///
/// Returns `None` when `text` is not bracket-wrapped, or when the
/// inside is empty. Arguments are split on whitespace at the *top* nesting
/// level only: `[...]`, `{...}` and `"..."` groups keep their inner
/// whitespace, so a nested command substitution like `[list [read $fd]]`
/// yields the single arg `[read $fd]` rather than the two broken halves
/// `[read` and `$fd]` (which would hide a taint source nested in an
/// argument). Callers that need full Tcl list quoting handle it upstream.
#[must_use]
pub fn parse_command_substitution(text: &str) -> Option<(String, Vec<String>)> {
    let stripped = text.trim();
    let inner = stripped.strip_prefix('[')?.strip_suffix(']')?.trim();
    let mut parts = split_top_level_words(inner).into_iter();
    let cmd = parts.next()?;
    let args: Vec<String> = parts.collect();
    Some((cmd, args))
}

/// Split a command body into words on top-level whitespace, keeping
/// `[...]` / `{...}` / `"..."` groups and backslash escapes intact.
fn split_top_level_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut brackets = 0i32;
    let mut braces = 0i32;
    let mut in_quote = false;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                cur.push(c);
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            '"' if brackets <= 0 && braces <= 0 => {
                in_quote = !in_quote;
                cur.push(c);
            }
            '[' if !in_quote => {
                brackets += 1;
                cur.push(c);
            }
            ']' if !in_quote => {
                brackets -= 1;
                cur.push(c);
            }
            '{' if !in_quote => {
                braces += 1;
                cur.push(c);
            }
            '}' if !in_quote => {
                braces -= 1;
                cur.push(c);
            }
            c if c.is_ascii_whitespace() && brackets <= 0 && braces <= 0 && !in_quote => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_var_bare_and_braced() {
        assert!(is_pure_var_ref("$x"));
        assert!(is_pure_var_ref("${x}"));
        assert!(is_pure_var_ref("$foo::bar"));
    }

    #[test]
    fn pure_var_rejects_extras() {
        assert!(!is_pure_var_ref("$x extra"));
        assert!(!is_pure_var_ref("hello$x"));
        assert!(!is_pure_var_ref("$x[nested]"));
        assert!(!is_pure_var_ref("\"$x\""));
        assert!(!is_pure_var_ref("x"));
    }

    #[test]
    fn pure_var_rejects_unbalanced_braces() {
        assert!(!is_pure_var_ref("${x}y"));
    }

    #[test]
    fn pure_var_accepts_array_index() {
        assert!(is_pure_var_ref("$arr(idx)"));
        assert!(is_pure_var_ref("$arr(key)"));
        assert!(is_pure_var_ref("${a(1)}"));
        // Backslash-escaped close paren stays inside the one index.
        assert!(is_pure_var_ref("$a(x\\)y)"));
    }

    #[test]
    fn fp_nab_12_escaped_paren_array_index_companions() {
        // FP-NAB-12: the hand-rolled parser consumes a backslash-escaped close
        // paren inside an
        // array index so the reference does not terminate at the first `)`.
        assert!(is_pure_var_ref(r"$a(x\)y)"));
        // Companion controls locking in overall parser correctness.
        assert!(is_pure_var_ref("$x"));
        assert!(is_pure_var_ref("${some name}"));
        assert!(is_pure_var_ref("$ns::x"));
        assert!(is_pure_var_ref("$arr(plain)"));
        // Inverse: an *unescaped* `)` terminates the index, leaving trailing
        // `y` as concatenated literal — NOT one pure var ref.
        assert!(!is_pure_var_ref("$a(x)y"));
    }

    #[test]
    fn pure_var_rejects_concatenation_and_trailing_text() {
        // Over-permissive cases the byte-set heuristic used to accept.
        assert!(!is_pure_var_ref("$x$y"));
        assert!(!is_pure_var_ref("$x.foo"));
        assert!(!is_pure_var_ref("$x_$y"));
        // Unterminated array index is not a pure reference.
        assert!(!is_pure_var_ref("$arr(idx"));
    }

    #[test]
    fn braced_whole_name_array_ref_detection() {
        assert!(is_braced_whole_name_array_ref("${a(1)}"));
        assert!(is_braced_whole_name_array_ref("${arr(key)}"));
        // Plain braced scalar is not an array-shaped ref.
        assert!(!is_braced_whole_name_array_ref("${foo}"));
        assert!(!is_braced_whole_name_array_ref("$arr(1)"));
        assert!(!is_braced_whole_name_array_ref("${a(1)}x"));
    }

    #[test]
    fn parse_command_substitution_basic() {
        let (cmd, args) = parse_command_substitution("[llength $x]").unwrap();
        assert_eq!(cmd, "llength");
        assert_eq!(args, vec!["$x".to_string()]);
    }

    #[test]
    fn parse_command_substitution_with_whitespace() {
        let (cmd, args) = parse_command_substitution("  [ set x 42 ]  ").unwrap();
        assert_eq!(cmd, "set");
        assert_eq!(args, vec!["x".to_string(), "42".into()]);
    }

    #[test]
    fn parse_command_substitution_rejects_non_bracketed() {
        assert!(parse_command_substitution("llength $x").is_none());
        assert!(parse_command_substitution("[empty ").is_none());
        assert!(parse_command_substitution("[]").is_none());
    }

    #[test]
    fn parse_command_substitution_no_args() {
        let (cmd, args) = parse_command_substitution("[pwd]").unwrap();
        assert_eq!(cmd, "pwd");
        assert!(args.is_empty());
    }
}
