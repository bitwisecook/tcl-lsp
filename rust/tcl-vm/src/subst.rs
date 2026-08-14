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

//! Runtime word substitution for literal `PUSH` operands.
//!
//! This compiler defers some substitution to the runtime: a word it cannot
//! fully inline is emitted as a literal, with variable substitutions normalised
//! to `${name}` and command substitutions left as `[script]` (a bare `$` — e.g.
//! from a braced word — stays literal). The VM resolves those at `PUSH` time,
//! mirroring the reference VM's `subst_command`. Whole-word simple variables are
//! already inlined to `loadStk`, so only `${…}` and `[…]` trigger here.
//!
//! Scope: `${name}` variable substitution and `[script]` command
//! substitution (via the injected `CompileService`). Backslash decoding and
//! array-element substitution are not yet implemented (a `\X` only escapes the next
//! char from being mis-read as a substitution trigger; it is not decoded).

use tcl_runtime_api::Code;

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// Find the matching `]` for the command substitution opening at `b[start]`,
/// honouring nested `[...]`, brace groups, and backslash escapes.
fn command_end(b: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut depth = 1usize;
    let mut brace = 0usize;
    while i < b.len() {
        match b[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'{' => brace += 1,
            b'}' if brace > 0 => brace -= 1,
            b'[' if brace == 0 => depth += 1,
            b']' if brace == 0 => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// If `word` is a single balanced brace group spanning the whole word
/// (`{` … matching `}` at the final byte), return the inner slice. A braced
/// literal suppresses *all* substitution, so the codegen marks such words by
/// keeping their outer `{...}` braces (see `emit_one_proc_def` /
/// `emit_cmd_subst_arg`); the runtime strips them here and returns the content
/// verbatim, mirroring the reference VM's `PUSH` handling.
fn whole_braced(word: &str) -> Option<&str> {
    let b = word.as_bytes();
    let n = b.len();
    if n < 2 || b[0] != b'{' || b[n - 1] != b'}' {
        return None;
    }
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < n {
        match b[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                // A premature return to depth 0 means the leading `{` does not
                // match the trailing `}`, so this is not a whole-word literal.
                if depth == 0 && i != n - 1 {
                    return None;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if depth == 0 {
        Some(&word[1..n - 1])
    } else {
        None
    }
}

fn read_var(vm: &mut Vm, name: &str) -> Result<Value, TclError> {
    if let Err(c) = vm.fire_var_traces(name, "read") {
        return Err(TclError::new(c.result.to_str().to_string()));
    }
    vm.get_var(name)
        .ok_or_else(|| TclError::new(format!("can't read \"{name}\": no such variable")))
}

/// Compile + run a command-substitution body, surfacing a non-`OK` completion
/// as an error rather than silently using the error message as the value.
fn eval_subst(vm: &mut Vm, inner: &str) -> Result<Value, TclError> {
    let c = vm.eval_source(inner)?;
    match c.code {
        tcl_runtime_api::Code::Ok => Ok(c.result),
        tcl_runtime_api::Code::Error => Err(TclError::new(c.result.to_str().to_string())),
        // A `break`/`continue`/`return` escaping the substitution carries its
        // own completion code out (so an enclosing loop / proc handles it),
        // rather than degrading to an error.
        other => Err(TclError::with_code(c.result.to_str().to_string(), other)),
    }
}

/// The `subst` command: perform variable, command, and backslash substitution
/// on `s` (each independently switchable). Returns the substituted string.
///
/// `subst` gives embedded command substitutions special control-flow handling,
/// distinct from ordinary `[...]` substitution and verified against C
/// (subst-8.x/10.x): a `break` stops substitution and yields the text
/// accumulated so far; a `continue` drops just that bracket's value and resumes;
/// a `return` (or any other non-error code) substitutes the result and resumes.
/// An unclosed `[` is a `missing close-bracket` error (subst-5.5).
#[allow(clippy::many_single_char_names)] // b/s/i/n name the byte buffer and scan cursor, mirroring the C subst loop.
pub fn subst_command(
    vm: &mut Vm,
    s: &str,
    backslashes: bool,
    commands: bool,
    variables: bool,
) -> Result<String, TclError> {
    let b = s.as_bytes();
    let n = b.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        match b[i] {
            b'\\' if backslashes => {
                // Decode exactly one backslash escape and advance past it. The
                // extent is the canonical `TclParseBackslash` rule *of the
                // emulated release* (8.6+ caps `\x` at two hex digits, 8.4/8.5
                // take every trailing one; the `\<newline>` continuation — LF,
                // CR, or CRLF — absorbs the following spaces/tabs), so the
                // decode always sees one whole escape and reads it the way the
                // pinned release would (issue #1479).
                let escapes = vm.escape_syntax();
                let end = tcl_syntax::backslash::escape_end_in(s, i, escapes);
                out.push_str(&tcl_syntax::backslash::decode_in(&s[i..end], escapes));
                i = end;
            }
            b'[' if commands => {
                // An unclosed `[` is a parse error reported before the bracket
                // body would run (subst-5.5/5.6/5.7).
                let end =
                    command_end(b, i).ok_or_else(|| TclError::new("missing close-bracket"))?;
                let c = vm.eval_source(&s[i + 1..end])?;
                match c.code {
                    // `return` / a custom code substitutes its result and resumes.
                    Code::Ok | Code::Return | Code::Other(_) => out.push_str(&c.result.to_str()),
                    Code::Continue => {} // drop this bracket's value, resume
                    Code::Break => return Ok(out), // stop, yield what we have
                    Code::Error => return Err(TclError::new(c.result.to_str().to_string())),
                }
                i = end + 1;
            }
            b'$' if variables && i + 1 < n => match subst_var(vm, s, i)? {
                VarFlow::Append(text, next) => {
                    out.push_str(&text);
                    i = next;
                }
                VarFlow::Skip(next) => i = next,
                VarFlow::Break => return Ok(out),
                VarFlow::Literal => {
                    out.push('$');
                    i += 1;
                }
            },
            _ => {
                // Copy one UTF-8 char.
                let ch = s[i..].chars().next().unwrap_or('\u{fffd}');
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    Ok(out)
}

/// Resumable state for a **yieldable** `subst` command activation:
/// the template, the scan cursor, the output accumulated so
/// far, and the three substitution switches. It lives on the `subst` frame (see
/// `crate::exec`), so it freezes with a suspended coroutine and resumes after
/// each `[…]` completes. Backslash / `$…` runs never yield, so they are scanned
/// natively; only a top-level `[…]` pauses the scan.
pub(crate) struct SubstState {
    pub(crate) template: String,
    pub(crate) cursor: usize,
    pub(crate) out: String,
    pub(crate) backslashes: bool,
    pub(crate) commands: bool,
    pub(crate) variables: bool,
}

impl SubstState {
    pub(crate) fn new(
        template: String,
        backslashes: bool,
        commands: bool,
        variables: bool,
    ) -> Self {
        Self {
            template,
            cursor: 0,
            out: String::new(),
            backslashes,
            commands,
            variables,
        }
    }
}

/// One step of the resumable `subst` scan.
pub(crate) enum SubstStep {
    /// The scan finished (end of template, or a `break` from a `$…` array index):
    /// the accumulated output is the `subst` result.
    Done(String),
    /// A top-level `[inner]` command substitution: compile + run `inner` on the
    /// explicit stack (yieldably). The cursor is left just past the `]`, so
    /// re-entry resumes after the bracket; the bracket's completion is folded back
    /// into `out` by the subst rules in `crate::exec`'s `unwind`.
    Bracket(String),
    /// A scan error (missing close-bracket, or a variable read / index error).
    Error(String),
}

/// Advance the resumable scan from `st.cursor`, appending literal / backslash /
/// `$…` runs (which never yield) into `st.out`, until it reaches a top-level `[`
/// (→ `Bracket`, cursor past the `]`), the end / a `break` (→ `Done`), or an
/// error. This is the resumable analogue of [`subst_command`]'s loop — the
/// literal/backslash/`$` arms match it exactly; only the `[…]` arm differs (it
/// pauses instead of calling `eval_source`).
pub(crate) fn subst_scan_step(vm: &mut Vm, st: &mut SubstState) -> SubstStep {
    // Clone the (immutable) template so the scan can borrow it while `st.out`
    // is mutated; restored to `st` before any pause/return.
    let template = st.template.clone();
    let b = template.as_bytes();
    let n = b.len();
    let mut i = st.cursor;
    let mut out = std::mem::take(&mut st.out);
    while i < n {
        match b[i] {
            b'\\' if st.backslashes => {
                let escapes = vm.escape_syntax();
                let end = tcl_syntax::backslash::escape_end_in(&template, i, escapes);
                out.push_str(&tcl_syntax::backslash::decode_in(
                    &template[i..end],
                    escapes,
                ));
                i = end;
            }
            b'[' if st.commands => {
                let Some(end) = command_end(b, i) else {
                    st.out = out;
                    st.cursor = i;
                    return SubstStep::Error("missing close-bracket".to_owned());
                };
                let inner = template[i + 1..end].to_owned();
                st.out = out;
                st.cursor = end + 1;
                return SubstStep::Bracket(inner);
            }
            b'$' if st.variables && i + 1 < n => match subst_var(vm, &template, i) {
                Ok(VarFlow::Append(text, next)) => {
                    out.push_str(&text);
                    i = next;
                }
                Ok(VarFlow::Skip(next)) => i = next,
                Ok(VarFlow::Break) => return SubstStep::Done(out),
                Ok(VarFlow::Literal) => {
                    out.push('$');
                    i += 1;
                }
                Err(e) => {
                    st.out = out;
                    st.cursor = i;
                    return SubstStep::Error(e.message);
                }
            },
            _ => {
                let ch = template[i..].chars().next().unwrap_or('\u{fffd}');
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    SubstStep::Done(out)
}

/// The outcome of substituting a top-level `$`-reference.
enum VarFlow {
    /// Append this text and resume at the byte offset.
    Append(String, usize),
    /// Drop the reference (a `continue` in its array index) and resume.
    Skip(usize),
    /// A `break` in the array index: stop substitution.
    Break,
    /// `$` not followed by a parseable name: emit it literally.
    Literal,
}

/// Substitute one `$name` / `${name}` / `$name(index)` reference at `s[at]`.
fn subst_var(vm: &mut Vm, s: &str, at: usize) -> Result<VarFlow, TclError> {
    let Some(vr) = parse_var_ref_parts(s, at) else {
        return Ok(VarFlow::Literal);
    };
    let Some(raw_index) = vr.index else {
        let v = read_var(vm, vr.base)?;
        return Ok(VarFlow::Append(v.to_str().to_string(), vr.next));
    };
    // `$name(index)` — the index is itself substituted (subst-4.3), and a
    // control-flow code from a command in the index decides the reference's
    // fate (subst-8.9).
    match subst_index(vm, raw_index)? {
        IndexFlow::Index(idx) => {
            let full = format!("{}({idx})", vr.base);
            let v = read_elem(vm, &full)?;
            Ok(VarFlow::Append(v.to_str().to_string(), vr.next))
        }
        IndexFlow::Substitute(v) => Ok(VarFlow::Append(v, vr.next)),
        IndexFlow::Skip => Ok(VarFlow::Skip(vr.next)),
        IndexFlow::Break => Ok(VarFlow::Break),
    }
}

/// The outcome of substituting an array index in `subst`.
enum IndexFlow {
    /// The fully-substituted index string.
    Index(String),
    /// A `return` (or other non-error code) in the index: replace the whole
    /// reference with this value (subst-8.9).
    Substitute(String),
    /// A `continue` in the index: drop the reference.
    Skip,
    /// A `break` in the index: stop substitution.
    Break,
}

/// Substitute an array index. Unlike the top level this is single-pass: the
/// first control-flow code from an embedded command stops the index and decides
/// the reference's fate.
fn subst_index(vm: &mut Vm, idx: &str) -> Result<IndexFlow, TclError> {
    let b = idx.as_bytes();
    let n = b.len();
    let mut out = String::new();
    let mut i = 0;
    while i < n {
        match b[i] {
            b'\\' => {
                let escapes = vm.escape_syntax();
                let end = tcl_syntax::backslash::escape_end_in(idx, i, escapes);
                out.push_str(&tcl_syntax::backslash::decode_in(&idx[i..end], escapes));
                i = end;
            }
            b'[' => {
                let end =
                    command_end(b, i).ok_or_else(|| TclError::new("missing close-bracket"))?;
                let c = vm.eval_source(&idx[i + 1..end])?;
                match c.code {
                    Code::Ok => {
                        out.push_str(&c.result.to_str());
                        i = end + 1;
                    }
                    Code::Return | Code::Other(_) => {
                        return Ok(IndexFlow::Substitute(c.result.to_str().to_string()));
                    }
                    Code::Continue => return Ok(IndexFlow::Skip),
                    Code::Break => return Ok(IndexFlow::Break),
                    Code::Error => return Err(TclError::new(c.result.to_str().to_string())),
                }
            }
            b'$' if i + 1 < n => {
                if let Some(vr) = parse_var_ref_parts(idx, i) {
                    let v = match vr.index {
                        None => read_var(vm, vr.base)?,
                        // A nested array index recurses; control flow from it
                        // propagates to decide the outer reference's fate.
                        Some(inner) => match subst_index(vm, inner)? {
                            IndexFlow::Index(k) => read_elem(vm, &format!("{}({k})", vr.base))?,
                            other => return Ok(other),
                        },
                    };
                    out.push_str(&v.to_str());
                    i = vr.next;
                } else {
                    out.push('$');
                    i += 1;
                }
            }
            _ => {
                let ch = idx[i..].chars().next().unwrap_or('\u{fffd}');
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    Ok(IndexFlow::Index(out))
}

/// Read an array element `base(key)` (firing read traces) with the standard
/// "no such variable" error. `var_get` resolves the `base(key)` form.
fn read_elem(vm: &mut Vm, full: &str) -> Result<Value, TclError> {
    if let Err(c) = vm.fire_var_traces(full, "read") {
        return Err(TclError::new(c.result.to_str().to_string()));
    }
    vm.var_get(full)
        .ok_or_else(|| TclError::new(format!("can't read \"{full}\": no such variable")))
}

/// A parsed `$`-reference split into its base name and optional raw array index.
struct VarRef<'a> {
    /// The `$name` / `${name}` base variable (or array) name.
    base: &'a str,
    /// The raw (unsubstituted) array index span, for `$name(index)`.
    index: Option<&'a str>,
    /// Byte offset just past the whole reference.
    next: usize,
}

/// Parse a `$`-variable reference starting at `s[at]` (`$name`, `${name}`,
/// `$name(idx)`).
fn parse_var_ref_parts(s: &str, at: usize) -> Option<VarRef<'_>> {
    let b = s.as_bytes();
    let n = b.len();
    if b.get(at + 1) == Some(&b'{') {
        let rel = s[at + 2..].find('}')?;
        let close = at + 2 + rel;
        return Some(VarRef {
            base: &s[at + 2..close],
            index: None,
            next: close + 1,
        });
    }
    let start = at + 1;
    let mut j = start;
    while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
        j += 1;
    }
    // Namespace separators `::`.
    while j + 1 < n && b[j] == b':' && b[j + 1] == b':' {
        j += 2;
        while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
            j += 1;
        }
    }
    if j == start {
        return None;
    }
    // Optional array index `(...)`.
    if j < n
        && b[j] == b'('
        && let Some(rel) = s[j..].find(')')
    {
        let close = j + rel;
        return Some(VarRef {
            base: &s[start..j],
            index: Some(&s[j + 1..close]),
            next: close + 1,
        });
    }
    Some(VarRef {
        base: &s[start..j],
        index: None,
        next: j,
    })
}

/// Substitute a literal word, returning its value. Pure single `${…}` / `[…]`
/// words return the underlying value (type-preserving); mixed words build a
/// string.
pub fn subst_word(word: &str, vm: &mut Vm) -> Result<Value, TclError> {
    let b = word.as_bytes();
    let n = b.len();

    // A whole-word braced literal suppresses all substitution: strip the outer
    // braces and return the content verbatim.
    if let Some(inner) = whole_braced(word) {
        return Ok(Value::string(inner));
    }

    // Fast path: the whole word is one command substitution.
    if b.first() == Some(&b'[')
        && let Some(end) = command_end(b, 0)
        && end == n - 1
    {
        return eval_subst(vm, &word[1..end]);
    }
    // Fast path: the whole word is one `${name}`.
    if n >= 3
        && b[0] == b'$'
        && b[1] == b'{'
        && let Some(rel) = word[2..].find('}')
        && 2 + rel == n - 1
    {
        return read_var(vm, &word[2..2 + rel]);
    }
    // No substitution triggers: the literal is its own value.
    if !word.contains("${") && !word.contains('[') {
        return Ok(Value::string(word));
    }

    // General scan: copy literal runs (backslash-decoded), substituting `${…}`
    // and `[…]`. Literal runs carry escapes the codegen left to prevent re-
    // substitution (`\$`/`\[`) or genuine escapes (`\n`, `\t`, …); decode them.
    let escapes = vm.escape_syntax();
    let mut out = String::with_capacity(n);
    let mut i = 0usize;
    let mut lit = 0usize;
    while i < n {
        match b[i] {
            // `\X` is a literal escape, not a trigger; skip it in the scan (it
            // is decoded with the surrounding literal run when copied).
            b'\\' => i = (i + 2).min(n),
            b'$' if i + 1 < n && b[i + 1] == b'{' => {
                out.push_str(&tcl_syntax::backslash::decode_in(&word[lit..i], escapes));
                if let Some(rel) = word[i + 2..].find('}') {
                    let close = i + 2 + rel;
                    let v = read_var(vm, &word[i + 2..close])?;
                    out.push_str(&v.to_str());
                    i = close + 1;
                } else {
                    out.push('$');
                    i += 1;
                }
                lit = i;
            }
            b'[' => {
                if let Some(end) = command_end(b, i) {
                    out.push_str(&tcl_syntax::backslash::decode_in(&word[lit..i], escapes));
                    let v = eval_subst(vm, &word[i + 1..end])?;
                    out.push_str(&v.to_str());
                    i = end + 1;
                    lit = i;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out.push_str(&tcl_syntax::backslash::decode_in(&word[lit..n], escapes));
    Ok(Value::string(out))
}

#[cfg(test)]
mod tests {
    use super::{command_end, subst_command, whole_braced};
    use crate::interp::Vm;

    /// `subst_command` with only backslash substitution enabled.
    fn subst_backslashes(template: &str) -> String {
        let mut vm = Vm::new();
        subst_command(&mut vm, template, true, false, false).expect("subst")
    }

    #[test]
    fn subst_hex_escape_consumes_exactly_two_digits() {
        // Tcl 9 caps `\x` at two hex digits (`TclParseBackslash`): `\x41BC`
        // is `A` + literal `BC`, never a wider code point or the 8.x
        // last-two-digits reading.
        assert_eq!(subst_backslashes(r"\x41BC"), "ABC");
        assert_eq!(subst_backslashes(r"\x4142"), "A42");
    }

    #[test]
    fn subst_backslash_newline_collapses_crlf_like_lf() {
        // `\<LF>`, `\<CR>`, and `\<CRLF>` each collapse — together with the
        // spaces/tabs after the newline — to a single space, matching what C
        // Tcl produces for a continuation (`TclParseBackslash`, with CRLF
        // normalised the way its channel layer would).
        assert_eq!(subst_backslashes("a\\\n   b"), "a b");
        assert_eq!(subst_backslashes("a\\\r\n\t b"), "a b");
        assert_eq!(subst_backslashes("a\\\rb"), "a b");
        // FP guard: `\\` is an escaped backslash, so the newline after it is
        // real content, not a continuation.
        assert_eq!(subst_backslashes("x\\\\\r\ny"), "x\\\r\ny");
    }

    #[test]
    fn command_end_finds_matching_bracket() {
        // Flat: the lone `]` closes the substitution.
        assert_eq!(command_end(b"[set x]", 0), Some(6));
        // Nested `[...]` raises/lowers depth; the outer `]` matches.
        assert_eq!(command_end(b"[a [b] c]", 0), Some(8));
    }

    #[test]
    fn command_end_ignores_brackets_inside_braces() {
        // A `]` inside a brace group is literal and must not close the subst.
        // `[set x {]}]`: the `]` at index 8 is brace-protected; index 10 closes.
        assert_eq!(command_end(b"[set x {]}]", 0), Some(10));
    }

    #[test]
    fn command_end_skips_backslash_escapes_and_reports_unbalanced() {
        // `\]` is an escaped bracket, not the closer; the real `]` is at 5.
        assert_eq!(command_end(br"[a\]b]", 0), Some(5));
        // No closing bracket at all.
        assert_eq!(command_end(b"[a b", 0), None);
    }

    #[test]
    fn whole_braced_strips_a_balanced_whole_word_group() {
        assert_eq!(whole_braced("{abc}"), Some("abc"));
        assert_eq!(whole_braced("{}"), Some("")); // empty group
        // Nested balanced braces stay inside the returned content.
        assert_eq!(whole_braced("{a {b} c}"), Some("a {b} c"));
    }

    #[test]
    fn whole_braced_rejects_non_whole_or_unbalanced_words() {
        // The first `}` closes the leading `{` before the word ends.
        assert_eq!(whole_braced("{a} {b}"), None);
        // Not brace-wrapped / no closer / too short.
        assert_eq!(whole_braced("abc"), None);
        assert_eq!(whole_braced("{abc"), None);
        assert_eq!(whole_braced(""), None);
        // An escaped brace does not count toward depth, so the group is whole.
        assert_eq!(whole_braced(r"{a\}b}"), Some(r"a\}b"));
    }
}
