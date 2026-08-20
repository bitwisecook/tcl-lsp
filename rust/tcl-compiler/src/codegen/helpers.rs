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

//! Pure utility functions for compile-time folding.
//!
//! These are standalone helpers with no emitter state — used by every
//! other codegen submodule.

/// Tcl 9.0 default trim characters — pushed when `string trim` is
/// called without an explicit chars argument.  Includes ASCII
/// whitespace, NUL, and all Unicode category Zs space separators.
pub const DEFAULT_TRIM_CHARS: &str = "\t\n\x0b\x0c\r \
    \x00\u{0085}\u{00a0}\
    \u{1680}\u{180e}\
    \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\
    \u{2008}\u{2009}\u{200a}\u{200b}\u{2028}\u{2029}\u{202f}\u{205f}\
    \u{3000}\u{feff}";

/// Split a constant Tcl list string into elements (compile-time only).
///
/// Handles braces, quotes, and backslash escaping.  Does not validate
/// syntax strictly — used only for known-good constant arguments.
///
/// Split a list into each element's **raw** text — braces / quotes stripped but
/// backslashes *not* decoded — so a split-then-[`tcl_list_element`] round-trip
/// re-emits the original literal rather than a decoded value.
///
/// Thin wrapper over the shared grammar
/// [`tcl_syntax::list::split_list_raw_lenient`]: the raw (non-decoding) split,
/// tolerant of a malformed tail. [`split_list_values`] is the decoding sibling
/// for when the runtime *value* is wanted instead.
#[must_use]
pub fn split_list_simple(text: &str) -> Vec<String> {
    tcl_syntax::list::split_list_raw_lenient(text)
        .into_iter()
        .map(ToOwned::to_owned)
        .collect()
}

/// Split a list like [`split_list_simple`], but return each element's decoded
/// *value*: braced words are kept verbatim while quoted and bare words are run
/// through backslash substitution. This is what the runtime `list` command sees
/// as its arguments, so constant-folding `[list …]` must decode the same way
/// (e.g. `"\x00"` → a NUL byte) before re-quoting each element.
///
/// Thin wrapper over the shared grammar
/// [`tcl_syntax::list::split_list_lenient`], whose literal/decoded policy (brace
/// verbatim, else `backslash_subst`) is exactly this.
///
/// Shared with SCCP's `foreach`/`lmap` constant-folding
/// ([`crate::sccp::extract_foreach_elements`] and friends): iterating a literal
/// list must split it with Tcl list semantics — `{a {b c} d}` is three
/// elements (`a`, `b c`, `d`), not four whitespace runs — so the loop variable
/// folds to the right CONSTSET.
pub(crate) fn split_list_values(text: &str) -> Vec<String> {
    tcl_syntax::list::split_list_lenient(text)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// Compute the Tcl string hash (matches `Tcl_HashString`).
#[must_use]
pub fn tcl_string_hash(s: &str) -> u32 {
    let mut h: u32 = 0;
    for ch in s.bytes() {
        h = h
            .wrapping_add(h.wrapping_shl(3))
            .wrapping_add(u32::from(ch));
    }
    h
}

/// Return jump-table entries in Tcl hash-table iteration order.
///
/// Tcl hash tables iterate buckets 0..N-1 and within each bucket
/// entries are in LIFO order (most recently inserted first).
/// The initial bucket count is 4.
#[must_use]
pub fn tcl_hash_table_order(entries: &[(String, String)]) -> Vec<(String, String)> {
    let mut num_buckets: usize = 4;
    // Grow buckets if needed (Tcl doubles when entries >= 3 * buckets).
    while entries.len() >= 3 * num_buckets {
        num_buckets *= 2;
    }

    // Assign entries to buckets in insertion order.
    let mut buckets: HashMap<usize, Vec<(String, String)>> = HashMap::new();
    for item in entries {
        let bucket = (tcl_string_hash(&item.0) as usize) & (num_buckets - 1);
        buckets.entry(bucket).or_default().push(item.clone());
    }

    // Iterate buckets 0..N-1; within each bucket reverse (LIFO).
    let mut result = Vec::with_capacity(entries.len());
    for b in 0..num_buckets {
        if let Some(bucket_entries) = buckets.get(&b) {
            for item in bucket_entries.iter().rev() {
                result.push(item.clone());
            }
        }
    }
    result
}

use std::collections::HashMap;

use tcl_dialect::EscapeSyntax;

/// Format a string as a canonical Tcl list element (`Tcl_ConvertElement`).
///
/// Delegates to the shared [`tcl_syntax::list::list_element`] (now also used by
/// the runtime and the registry const-folder) — the single Tcl-faithful
/// quoter, correct on the leading-`#` and control-char cases this local copy
/// previously mis-quoted.
#[must_use]
pub fn tcl_list_element(s: &str) -> String {
    tcl_syntax::list::list_element(s)
}

/// A part of a parsed substitution template.
///
/// There is deliberately **no** variant for the retired `$={name}`
/// braced-scalar marker: nothing in this workspace ever emitted that spelling,
/// so every word that reached its decoder came from the user's own source,
/// where `$={y}` is plain literal text in every supported release (issue
/// #1617).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstPart {
    /// Literal text.
    Lit(String),
    /// Variable reference (`$varname`).
    Var(String),
    /// Command substitution (`[cmd ...]`).
    Cmd(String),
}

/// If `bytes[i]` opens an array index (`(`), advance past the matching `)` —
/// honouring nested `[...]` command substitutions inside the index — and return
/// the new position. Otherwise return `i` unchanged.
fn consume_array_index(bytes: &[u8], mut i: usize) -> usize {
    let n = bytes.len();
    if i >= n || bytes[i] != b'(' {
        return i;
    }
    let mut depth: u32 = 0;
    i += 1;
    while i < n {
        match bytes[i] {
            b'[' => depth += 1,
            b']' if depth > 0 => depth -= 1,
            b')' if depth == 0 => return i + 1,
            _ => {}
        }
        i += 1;
    }
    i
}

/// Flush the raw literal run `template[lit_start..end]` (if non-empty) as a
/// [`SubstPart::Lit`], decoding it through the shared full backslash decoder
/// [`tcl_lexer::backslash_subst_in`] — the same decoder `expressions.rs`'s
/// no-substitution literal branch uses. That decoder is UTF-8-aware and
/// handles every escape form (`\xNN`, `\uNNNN`, `\UNNNNNNNN`, octal `\NNN`,
/// line continuation) under the release being compiled for, unlike the
/// seven-letter-escape hand-roll this replaced.
fn flush_subst_lit(
    parts: &mut Vec<SubstPart>,
    template: &str,
    lit_start: usize,
    end: usize,
    escapes: EscapeSyntax,
) {
    if lit_start < end {
        parts.push(SubstPart::Lit(
            tcl_lexer::backslash_subst_in(&template[lit_start..end], escapes).into_owned(),
        ));
    }
}

/// Parse a substitution template into parts.
///
/// Returns `None` if the template contains constructs we cannot
/// inline (unbalanced brackets, bare `$` without a name, etc.).
///
/// Only scans for `$`/`[` substitution triggers; literal text (including any
/// backslash escapes within it) is left raw in the source and decoded in one
/// pass by [`flush_subst_lit`] when a literal run ends. A backslash escape is
/// always skipped as a unit via [`tcl_lexer::backslash_escape_end_in`] before
/// the trigger check runs, so an escaped `\$`/`\[` decodes to a literal `$`/`[`
/// and never starts a substitution.
///
/// `escapes` is the target release's backslash grammar — the skip width and the
/// decoded value must come from the same release, so both take it.
///
/// `braced_var` is the same release's `${…}` close rule, resolved through the
/// shared owner [`tcl_lexer::braced_var_name_end`]. It is a second grammar
/// fact about the same target and travels beside `escapes` for the same
/// reason: this decoder hard-coded the 8.x first-`}` rule at every release
/// while `values::parse_simple_var_ref` hard-coded the 9.x nesting rule, so
/// the compiled-word path was wrong in both directions at once (issue #1568).
#[must_use]
pub fn parse_subst_template(
    template: &str,
    escapes: EscapeSyntax,
    braced_var: tcl_dialect::BracedVarStyle,
) -> Option<Vec<SubstPart>> {
    let bytes = template.as_bytes();
    let n = bytes.len();
    let mut parts = Vec::new();
    let mut i = 0;
    let mut lit_start = 0;

    while i < n {
        let ch = bytes[i];

        if ch == b'\\' {
            i = tcl_lexer::backslash_escape_end_in(template, i, escapes);
            continue;
        }

        if ch == b'[' {
            // Command substitution
            flush_subst_lit(&mut parts, template, lit_start, i, escapes);
            let mut depth: u32 = 0;
            let start = i;
            while i < n {
                if bytes[i] == b'[' {
                    depth += 1;
                } else if bytes[i] == b']' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            if depth != 0 {
                return None;
            }
            parts.push(SubstPart::Cmd(template[start..i].to_owned()));
            lit_start = i;
            continue;
        }

        if ch == b'$' {
            flush_subst_lit(&mut parts, template, lit_start, i, escapes);
            i += 1;
            if i >= n {
                return None;
            }
            if bytes[i] == b'{' {
                // Braced variable: ${name}, closed by the target release's
                // `Tcl_ParseVarName` rule. This used to be `find('}')` — the
                // 8.x first-close rule applied at every release, disagreeing
                // with `values::parse_simple_var_ref`'s 9.x nesting rule on the
                // very same encoding (issue #1568).
                let end = match tcl_lexer::braced_var_name_end(bytes, i + 1, braced_var) {
                    tcl_lexer::BracedVarEnd::Closed(end) => end,
                    tcl_lexer::BracedVarEnd::Unterminated => return None,
                };
                parts.push(SubstPart::Var(template[i + 1..end].to_owned()));
                i = end + 1;
            } else {
                // Bare variable: $varname, optionally with an array index
                // `$arr(index)` whose index may itself contain substitutions.
                // A `:` is a name character only as part of a `::` namespace
                // separator (matching the lexer, `Tcl_ParseVarName`): a lone
                // colon ends the name, so `$action:` is `$action` then a literal
                // `:`, not a variable `action:`. Once a `::` starts, the whole
                // colon run is consumed (`$a:::b` names `a:::b`).
                let start = i;
                while i < n {
                    if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
                        i += 1;
                    } else if bytes[i] == b':' && i + 1 < n && bytes[i + 1] == b':' {
                        while i < n && bytes[i] == b':' {
                            i += 1;
                        }
                    } else {
                        break;
                    }
                }
                if i == start {
                    return None;
                }
                i = consume_array_index(bytes, i);
                parts.push(SubstPart::Var(template[start..i].to_owned()));
            }
            lit_start = i;
            continue;
        }

        i += 1;
    }

    flush_subst_lit(&mut parts, template, lit_start, n, escapes);

    if parts.is_empty() { None } else { Some(parts) }
}

/// Convert a simple anchored regex to an equivalent glob pattern.
///
/// Handles:
/// - `{hello}` → `*hello*` (unanchored)
/// - `{^abc}` → `abc*` (left-anchored)
/// - `{abc$}` → `*abc` (right-anchored)
/// - `{^abc$}` → `abc` (fully anchored)
///
/// Returns `None` if the pattern contains regex metacharacters that
/// cannot be expressed as a glob.
#[must_use]
pub fn regexp_to_glob(pattern: &str) -> Option<String> {
    const REGEX_SPECIAL: &[u8] = b".+*?[](){}|\\";

    let pattern = pattern
        .strip_prefix('{')
        .and_then(|p| p.strip_suffix('}'))
        .unwrap_or(pattern);

    if pattern.is_empty() {
        return None;
    }

    let left_anchor = pattern.starts_with('^');
    let right_anchor = pattern.ends_with('$');

    let core = &pattern[usize::from(left_anchor)..pattern.len() - usize::from(right_anchor)];
    if core.is_empty() {
        return None;
    }

    if core.bytes().any(|b| REGEX_SPECIAL.contains(&b)) {
        return None;
    }

    Some(match (left_anchor, right_anchor) {
        (true, true) => core.to_owned(),
        (true, false) => format!("{core}*"),
        (false, true) => format!("*{core}"),
        (false, false) => format!("*{core}*"),
    })
}

/// Constant-fold a command with all-literal args to a single string.
///
/// `prefix` is the command text including trailing space, e.g.
/// `"list "` or `"dict create "`.  Returns `None` when the value
/// doesn't match or contains substitutions.
#[must_use]
pub fn fold_cmd_args(value: &str, prefix: &str) -> Option<String> {
    // `Tcl_Merge`, not a per-element `Tcl_ConvertElement` loop: a leading `#`
    // is comment-unsafe only in list position 0, so mapping the single-element
    // quoter over every argument rendered `[list a #]` as `a {#}` where both
    // tclsh oracles print `a #` (issues #1439 / #1608).
    Some(tcl_syntax::list::join_list(fold_cmd_arg_values(
        value, prefix,
    )?))
}

/// [`fold_cmd_args`]'s foldability check, stopping at the **argument values**
/// rather than the rendered list.
///
/// Split out because `dict create` cannot simply join its arguments: it has to
/// collapse duplicate keys first (issue #1427), which is a decision about
/// values, not about rendered list elements.
#[must_use]
fn fold_cmd_arg_values(value: &str, prefix: &str) -> Option<Vec<String>> {
    let full_prefix = format!("[{prefix}");
    let inner = value
        .strip_prefix(&full_prefix)
        .and_then(|s| s.strip_suffix(']'))?;

    // Canonical C-rule continuation collapse (`\<LF>`/`\<CR>`/`\<CRLF>` +
    // following spaces/tabs → one space) — UTF-8-safe, unlike the retired
    // local byte-by-byte copy, which pushed each byte through `char::from`
    // and mangled multi-byte characters.
    let inner = tcl_syntax::backslash::collapse_brace_continuations_str(inner);
    let inner = inner.as_ref();

    // Cannot fold across a `{*}` argument expansion: it turns one braced word
    // into *several* arguments, so a naive literal fold would mis-read `{*}` as
    // the braced word `*`. Bail to the normal (expansion-aware) codegen.
    if has_expand_marker(inner) {
        return None;
    }

    // Cannot fold if UNBRACED arguments contain substitutions.
    let mut depth: i32 = 0;
    for ch in inner.bytes() {
        match ch {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'$' | b'[' if depth == 0 => return None,
            _ => {}
        }
    }

    // Parse the literal arguments and re-format as a canonical Tcl list. Each
    // argument's *value* is what `list` quotes: braced words are verbatim, while
    // quoted and bare words are backslash-decoded first (so `"\x00"` becomes a
    // NUL byte and `"a\tb"` an embedded tab, not the literal escape text).
    Some(split_list_values(inner))
}

/// Whether `s` contains a `{*}` argument-expansion operator: a `{*}` at a word
/// boundary (start of string or after whitespace) immediately followed by a
/// non-separator (Tcl 8.5+ `expand_syntax`). A `{*}` followed by whitespace —
/// or one embedded mid-word / inside a deeper word — is the literal braced word
/// `*`, not an expansion.
fn has_expand_marker(s: &str) -> bool {
    let b = s.as_bytes();
    let is_sep = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r');
    let mut i = 0;
    let mut word_start = true;
    while i < b.len() {
        if is_sep(b[i]) {
            word_start = true;
            i += 1;
            continue;
        }
        if word_start
            && b[i] == b'{'
            && b.get(i + 1) == Some(&b'*')
            && b.get(i + 2) == Some(&b'}')
            && b.get(i + 3).is_some_and(|&c| !is_sep(c))
        {
            return true;
        }
        word_start = false;
        i += 1;
    }
    false
}

/// Constant-fold `[list arg1 arg2 ...]` to the result string.
#[must_use]
pub fn fold_list_cmd(value: &str) -> Option<String> {
    fold_cmd_args(value, "list ")
}

/// Constant-fold `[dict create k v ...]` to the result string.
///
/// **Not** a plain list join of the arguments. `dict create` builds a
/// dictionary, so it canonicalises: a repeated key keeps its
/// **first-occurrence position** and its **last value**
/// (`Tcl_DictObjPut` over `DictCreateCmd`'s argument walk).
///
/// The walk is the shared owner's,
/// [`tcl_syntax::value::canonical_dict_slots`] — the same function behind the
/// runtime seam `ValueOps::dict_pairs` and the registry's `dict` const-folds.
/// This site used to carry its own (`Vec::iter_mut().find`, O(N²)) copy; three
/// correct copies and one place the rule had been missed is what issue #1608
/// was filed about, and the cross-crate `dict_canonicalisation_parity` gate now
/// fails if a copy reappears and diverges. Only the *rendering* is local: a
/// folded `dict create` is re-quoted element by element here.
///
/// Joining instead froze `[dict create a 1 a 2]` into the literal `a 1 a 2`,
/// whose *string representation* is a dict nothing canonicalises — `dict size`
/// and `dict get` read it correctly (they re-canonicalise on the way in), but
/// `puts`/`string length` and every other string consumer saw the duplicate.
/// The bug was invisible whenever a value was non-literal, because that
/// defeats the fold and the correct runtime path runs (issue #1427).
///
/// An odd argument count is `wrong # args`, which only the runtime should
/// report, so it declines to fold.
#[must_use]
pub fn fold_dict_create_cmd(value: &str) -> Option<String> {
    let args = fold_cmd_arg_values(value, "dict create ")?;
    if args.len() % 2 != 0 {
        return None;
    }
    let canonical: Vec<&str> =
        tcl_syntax::value::canonical_dict_slots(args.iter().step_by(2).map(String::as_str))
            .into_iter()
            .flat_map(|(key_slot, value_slot)| {
                [
                    args[key_slot * 2].as_str(),
                    args[value_slot * 2 + 1].as_str(),
                ]
            })
            .collect();
    // Rendered by the shared `Tcl_Merge` owner — the position-0-only `#` rule
    // matters here too (`[dict create a #]` is `a #`, not `a {#}`).
    Some(tcl_syntax::list::join_list(canonical))
}

/// Constant-fold `[format "..." arg ...]` → result string.
///
/// Only handles simple `%s` and `%d` conversions with literal args.
/// Returns `None` if the format cannot be folded.
#[must_use]
pub fn try_format_fold(value: &str) -> Option<String> {
    let inner = value.strip_prefix("[format ")?.strip_suffix(']')?;

    // Parse the command parts (format string + arguments). Bails (`None`) when
    // any word is a runtime substitution (`$var` / `[cmd]`) rather than a
    // compile-time constant — folding those would freeze the literal source
    // text (`$cmd`) into the result instead of its value.
    let parts = parse_format_parts(inner)?;
    if parts.is_empty() {
        return None;
    }

    let fmt = &parts[0];
    let args = &parts[1..];

    // Process format specifiers
    let fmt_bytes = fmt.as_bytes();
    let mut result = String::new();
    let mut ai = 0;
    let mut fi = 0;

    while fi < fmt_bytes.len() {
        if fmt_bytes[fi] == b'%' && fi + 1 < fmt_bytes.len() {
            match fmt_bytes[fi + 1] {
                b's' if ai < args.len() => {
                    result.push_str(&args[ai]);
                    ai += 1;
                    fi += 2;
                }
                b'd' if ai < args.len() => {
                    let n: i64 = args[ai].parse().ok()?;
                    result.push_str(&n.to_string());
                    ai += 1;
                    fi += 2;
                }
                b'%' => {
                    result.push('%');
                    fi += 2;
                }
                _ => return None,
            }
        } else {
            result.push(char::from(fmt_bytes[fi]));
            fi += 1;
        }
    }

    Some(result)
}

/// Parse the argument parts of a `format` command. Returns `None` when a word
/// is a runtime substitution (a quoted or bare word containing an unescaped `$`
/// or `[`) — such a word is not a compile-time constant and must not be folded.
/// Braced words are always literal (Tcl braces suppress substitution).
fn parse_format_parts(inner: &str) -> Option<Vec<String>> {
    let bytes = inner.as_bytes();
    let mut parts = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'"' => {
                // Quoted string — subject to substitution, so a `$`/`[` makes it
                // non-constant (an escaped `\$`/`\[` stays literal).
                i += 1;
                let mut buf = String::new();
                let mut subst = false;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        if i + 1 < bytes.len() {
                            buf.push(char::from(bytes[i]));
                            buf.push(char::from(bytes[i + 1]));
                            i += 2;
                        } else {
                            buf.push(char::from(bytes[i]));
                            i += 1;
                        }
                    } else {
                        if matches!(bytes[i], b'$' | b'[') {
                            subst = true;
                        }
                        buf.push(char::from(bytes[i]));
                        i += 1;
                    }
                }
                if i < bytes.len() {
                    i += 1; // skip closing "
                }
                if subst {
                    return None;
                }
                // The word is a compile-time constant (no unescaped `$`/`[`); the
                // backslash sequences it still carries (`\"`, `\\`, `\t`, escaped
                // `\$`/`\[`, …) must be substituted to get the folded value —
                // otherwise `[format "x\"y"]` would freeze the literal `x\"y`.
                //
                // Release-blind by necessity: `try_format_fold` is also an SCCP
                // const-folder, which has no target release in hand, so this
                // reads under Tcl 9.0 — the documented `Unknown` posture. The
                // release-variant forms (`\x` with three or more hex digits,
                // `\U`, three-digit octal at or above `\40`) are the ones it
                // therefore folds under 9.0 for every dialect (issue #1479).
                parts.push(tcl_lexer::backslash_subst(&buf).into_owned());
            }
            b'{' => {
                // Braced string — always literal.
                let mut depth: u32 = 0;
                let start = i;
                while i < bytes.len() {
                    if bytes[i] == b'{' {
                        depth += 1;
                    } else if bytes[i] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    i += 1;
                }
                parts.push(inner[start + 1..i.saturating_sub(1)].to_owned());
            }
            _ => {
                // Bare word — a `$`/`[` makes it a substitution, not a constant.
                let start = i;
                while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                let word = &inner[start..i];
                if word.contains('$') || word.contains('[') {
                    return None;
                }
                parts.push(word.to_owned());
            }
        }
    }

    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`parse_subst_template`] under the release-blind Tcl 9.0 grammar — what
    /// these tests assert unless they name a release.
    fn template(text: &str) -> Option<Vec<SubstPart>> {
        parse_subst_template(
            text,
            EscapeSyntax::default(),
            tcl_dialect::BracedVarStyle::default(),
        )
    }

    #[test]
    fn subst_template_literals_decode_for_the_target_release() {
        // The compile target's escape grammar reaches the template parser, so
        // an 8.5 build freezes `B` where a 9.0 build freezes `A42`, and the
        // skip width used to find the next `$`/`[` trigger matches (#1479).
        let lit = |text: &str, escapes| match parse_subst_template(
            text,
            escapes,
            tcl_dialect::BracedVarStyle::default(),
        )
        .expect("template parses")
        .as_slice()
        {
            [SubstPart::Lit(s)] => s.clone(),
            other => panic!("expected one literal part, got {other:?}"),
        };
        assert_eq!(lit(r"\x4142", EscapeSyntax::Tcl84), "B");
        assert_eq!(lit(r"\x4142", EscapeSyntax::Tcl86), "A42");
        assert_eq!(lit(r"\x4142", EscapeSyntax::Tcl90), "A42");
        assert_eq!(lit(r"\U0001F600", EscapeSyntax::Tcl84), "U0001F600");
        assert_eq!(lit(r"\U0001F600", EscapeSyntax::Tcl90), "\u{1F600}");
        assert_eq!(lit(r"\400", EscapeSyntax::Tcl84), "\0");
        assert_eq!(lit(r"\400", EscapeSyntax::Tcl90), " 0");
    }

    // -- split_list_simple --

    #[test]
    fn split_list_simple_basic() {
        assert_eq!(split_list_simple("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn split_list_simple_braces() {
        assert_eq!(
            split_list_simple("{hello world} foo"),
            vec!["hello world", "foo"]
        );
    }

    #[test]
    fn split_list_simple_quotes() {
        assert_eq!(
            split_list_simple(r#""hello world" foo"#),
            vec!["hello world", "foo"]
        );
    }

    #[test]
    fn split_list_simple_nested_braces() {
        assert_eq!(split_list_simple("{a {b c}} d"), vec!["a {b c}", "d"]);
    }

    #[test]
    fn split_list_simple_empty() {
        assert!(split_list_simple("").is_empty());
        assert!(split_list_simple("   ").is_empty());
    }

    #[test]
    fn split_list_simple_backslash() {
        assert_eq!(split_list_simple(r"a\ b c"), vec![r"a\ b", "c"]);
    }

    // -- tcl_string_hash --

    #[test]
    fn tcl_string_hash_empty() {
        assert_eq!(tcl_string_hash(""), 0);
    }

    #[test]
    fn tcl_string_hash_deterministic() {
        let h1 = tcl_string_hash("hello");
        let h2 = tcl_string_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn tcl_string_hash_different() {
        assert_ne!(tcl_string_hash("a"), tcl_string_hash("b"));
    }

    // -- tcl_hash_table_order --

    #[test]
    fn hash_table_order_preserves_all() {
        let entries = vec![
            ("a".into(), "L1".into()),
            ("b".into(), "L2".into()),
            ("c".into(), "L3".into()),
        ];
        let ordered = tcl_hash_table_order(&entries);
        assert_eq!(ordered.len(), 3);
        // All original entries present (order may differ due to hashing)
        for e in &entries {
            assert!(ordered.contains(e));
        }
    }

    #[test]
    fn hash_table_order_bucket_growth() {
        // 12+ entries should trigger bucket growth
        let entries: Vec<(String, String)> = (0..15)
            .map(|i| (format!("key{i}"), format!("L{i}")))
            .collect();
        let ordered = tcl_hash_table_order(&entries);
        assert_eq!(ordered.len(), 15);
    }

    // -- tcl_list_element --

    #[test]
    fn tcl_list_element_empty() {
        assert_eq!(tcl_list_element(""), "{}");
    }

    #[test]
    fn tcl_list_element_simple() {
        assert_eq!(tcl_list_element("hello"), "hello");
    }

    #[test]
    fn tcl_list_element_space() {
        assert_eq!(tcl_list_element("hello world"), "{hello world}");
    }

    #[test]
    fn tcl_list_element_backslash_end() {
        // Ends with backslash → can't brace-quote → backslash quoting
        let result = tcl_list_element("foo\\");
        assert!(result.contains('\\'));
        assert!(!result.starts_with('{'));
    }

    // -- parse_subst_template --

    #[test]
    fn subst_template_lit_only() {
        let parts = template("hello").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("hello".into())]);
    }

    #[test]
    fn subst_template_var() {
        let parts = template("Hello, $name!").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], SubstPart::Lit("Hello, ".into()));
        assert_eq!(parts[1], SubstPart::Var("name".into()));
        assert_eq!(parts[2], SubstPart::Lit("!".into()));
    }

    #[test]
    fn subst_template_braced_var() {
        let parts = template("${foo}bar").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], SubstPart::Var("foo".into()));
        assert_eq!(parts[1], SubstPart::Lit("bar".into()));
    }

    #[test]
    fn subst_template_cmd() {
        let parts = template("x[cmd arg]y").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], SubstPart::Lit("x".into()));
        assert_eq!(parts[1], SubstPart::Cmd("[cmd arg]".into()));
        assert_eq!(parts[2], SubstPart::Lit("y".into()));
    }

    #[test]
    fn subst_template_backslash() {
        let parts = template(r"a\nb").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("a\nb".into())]);
    }

    #[test]
    fn subst_template_bare_dollar() {
        assert!(template("$").is_none());
    }

    #[test]
    fn subst_template_lone_colon_ends_var_name() {
        // A single `:` ends the variable name (it is a name char only in a `::`
        // pair): `$action:` is `$action` then a literal `:`, not a variable
        // `action:`. Regression for the whole-word command-substitution inline
        // path reading `can't read "action:"` (tcl-irule-test's orchestrator).
        assert_eq!(
            template("$action:"),
            Some(vec![
                SubstPart::Var("action".into()),
                SubstPart::Lit(":".into()),
            ])
        );
    }

    #[test]
    fn subst_template_namespace_qualified_var() {
        // `::` separators (and a `::`-run) stay part of the name, matching the
        // lexer / `Tcl_ParseVarName`.
        assert_eq!(
            template("$ns::v"),
            Some(vec![SubstPart::Var("ns::v".into())])
        );
        assert_eq!(template("$::g"), Some(vec![SubstPart::Var("::g".into())]));
    }

    #[test]
    fn subst_template_empty() {
        assert!(template("").is_none());
    }

    /// `$=` is not a substitution trigger in any release: `=` is not a name
    /// character, so `Tcl_ParseVarName` leaves the `$` literal. This decoder
    /// used to read `$={name}` as the compiler's "braced scalar" marker — a
    /// producer-less port artifact that only ever fired on the user's own text
    /// (issue #1617). Both tclsh oracles print `$={y}` for `puts $={y}`.
    #[test]
    fn subst_template_dollar_equals_is_not_a_marker() {
        // No name follows the `$`, so the word has no decomposable
        // substitution at all and the caller pushes it as a literal.
        assert!(template("$={a(1)}rest").is_none());
        assert!(template("$={y}").is_none());
        // The neighbouring real form is untouched.
        assert_eq!(template("${y}"), Some(vec![SubstPart::Var("y".into())]),);
    }

    // -- parse_subst_template: full backslash decoding (issue #1441) --

    #[test]
    fn subst_template_hex_escape_then_var() {
        // Regression for issue #1441: `\x41` was emitted as the literal text
        // `x41` instead of decoding to `A`.
        let parts = template(r"\x41$v").unwrap();
        assert_eq!(
            parts,
            vec![SubstPart::Lit("A".into()), SubstPart::Var("v".into())]
        );
    }

    #[test]
    fn subst_template_hex_escape_then_array_var() {
        // The exact issue #1441 reproducer shape: `"\x41$arr(0)"`.
        let parts = template(r"\x41$arr(0)").unwrap();
        assert_eq!(
            parts,
            vec![SubstPart::Lit("A".into()), SubstPart::Var("arr(0)".into()),]
        );
    }

    #[test]
    fn subst_template_octal_escape() {
        // Regression for issue #1441: `\101` was emitted as the literal text
        // `101` instead of decoding to `A`.
        let parts = template(r"\101").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("A".into())]);
    }

    #[test]
    fn subst_template_unicode_escape() {
        // Regression for issue #1441: `\uNNNN` fell to the `_ =>
        // char::from(next_ch)` branch and was emitted as literal `u00e9`
        // instead of decoding to `é`.
        let parts = template("\\u00e9").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("é".into())]);
    }

    #[test]
    fn subst_template_wide_unicode_escape() {
        let parts = template(r"\U0001F600").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("\u{1F600}".into())]);
    }

    #[test]
    fn subst_template_multibyte_literal_text_with_var() {
        // Regression for issue #1441: non-escape bytes were pushed one at a
        // time via `char::from(byte)`, mangling multi-byte UTF-8 (e.g. `é`)
        // into separate Latin-1 code points.
        let parts = template("café $name").unwrap();
        assert_eq!(
            parts,
            vec![
                SubstPart::Lit("café ".into()),
                SubstPart::Var("name".into()),
            ]
        );
    }

    #[test]
    fn subst_template_escaped_dollar_and_bracket_stay_literal() {
        // `\$` and `\[` must decode to a literal `$`/`[` and must NOT be
        // treated as a variable/command substitution trigger.
        let parts = template(r"\$foo \[bar\]").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("$foo [bar]".into())]);
    }

    #[test]
    fn subst_template_escaped_dollar_before_real_var() {
        // A real substitution after an escaped one still triggers correctly.
        let parts = template(r"\$$name").unwrap();
        assert_eq!(
            parts,
            vec![SubstPart::Lit("$".into()), SubstPart::Var("name".into())]
        );
    }

    // -- regexp_to_glob --

    #[test]
    fn regexp_to_glob_unanchored() {
        assert_eq!(regexp_to_glob("{hello}"), Some("*hello*".into()));
    }

    #[test]
    fn regexp_to_glob_left_anchored() {
        assert_eq!(regexp_to_glob("{^abc}"), Some("abc*".into()));
    }

    #[test]
    fn regexp_to_glob_right_anchored() {
        assert_eq!(regexp_to_glob("{abc$}"), Some("*abc".into()));
    }

    #[test]
    fn regexp_to_glob_fully_anchored() {
        assert_eq!(regexp_to_glob("{^abc$}"), Some("abc".into()));
    }

    #[test]
    fn regexp_to_glob_metachar() {
        assert_eq!(regexp_to_glob("{a.*b}"), None);
    }

    #[test]
    fn regexp_to_glob_empty() {
        assert_eq!(regexp_to_glob("{}"), None);
    }

    #[test]
    fn regexp_to_glob_without_braces() {
        assert_eq!(regexp_to_glob("^abc$"), Some("abc".into()));
    }

    // -- fold_cmd_args / fold_list_cmd / fold_dict_create_cmd --

    #[test]
    fn fold_list_cmd_basic() {
        assert_eq!(fold_list_cmd("[list a b c]"), Some("a b c".into()));
    }

    #[test]
    fn fold_list_cmd_braced() {
        assert_eq!(
            fold_list_cmd("[list {hello world} b]"),
            Some("{hello world} b".into())
        );
    }

    #[test]
    fn fold_list_cmd_substitution() {
        // Contains $ → cannot fold
        assert_eq!(fold_list_cmd("[list $x b]"), None);
    }

    #[test]
    fn fold_list_cmd_no_match() {
        assert_eq!(fold_list_cmd("not a list"), None);
    }

    #[test]
    fn fold_dict_create_basic() {
        assert_eq!(
            fold_dict_create_cmd("[dict create a 1 b 2]"),
            Some("a 1 b 2".into())
        );
    }

    // -- try_format_fold --

    #[test]
    fn format_fold_simple_s() {
        assert_eq!(
            try_format_fold("[format \"%s world\" hello]"),
            Some("hello world".into())
        );
    }

    #[test]
    fn format_fold_simple_d() {
        assert_eq!(try_format_fold("[format \"%d\" 42]"), Some("42".into()));
    }

    #[test]
    fn format_fold_percent_escape() {
        assert_eq!(try_format_fold("[format \"100%%\"]"), Some("100%".into()));
    }

    #[test]
    fn format_fold_no_match() {
        assert_eq!(try_format_fold("not format"), None);
    }

    #[test]
    fn format_fold_bails_on_variable_arg() {
        // A `$var` argument is a runtime substitution, not a constant — folding
        // it would freeze the literal text `$cmd` into the result.
        assert_eq!(try_format_fold("[format {x %s} $cmd]"), None);
        assert_eq!(try_format_fold("[format {%s} $cmd]"), None);
    }

    #[test]
    fn format_fold_bails_on_command_sub_arg() {
        assert_eq!(try_format_fold("[format {%s} [id]]"), None);
    }

    #[test]
    fn format_fold_bails_on_quoted_subst() {
        assert_eq!(try_format_fold("[format {%s} \"$x\"]"), None);
    }

    #[test]
    fn format_fold_multiline_template_constant_args() {
        // A multi-line braced template with constant args still folds.
        assert_eq!(try_format_fold("[format {a\n%s} hi]"), Some("a\nhi".into()));
    }
}
