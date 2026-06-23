//! Runtime word substitution for literal `PUSH` operands.
//!
//! This compiler defers some substitution to the runtime: a word it cannot
//! fully inline is emitted as a literal, with variable substitutions normalised
//! to `${name}` and command substitutions left as `[script]` (a bare `$` — e.g.
//! from a braced word — stays literal). The VM resolves those at `PUSH` time,
//! mirroring the reference VM's `subst_command`. Whole-word simple variables are
//! already inlined to `loadStk`, so only `${…}` and `[…]` trigger here.
//!
//! M1 scope: `${name}` variable substitution and `[script]` command
//! substitution (via the injected `CompileService`). Backslash decoding and
//! array-element substitution are deferred to M2 (a `\X` only escapes the next
//! char from being mis-read as a substitution trigger; it is not decoded).

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
#[allow(clippy::many_single_char_names)]
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
                // Decode the escape (`tcl_lexer::backslash_subst` over the run).
                if i + 1 < n {
                    let decoded = tcl_syntax::backslash::decode(&s[i..i + 2]);
                    out.push_str(&decoded);
                    i += 2;
                } else {
                    out.push('\\');
                    i += 1;
                }
            }
            b'[' if commands => {
                if let Some(end) = command_end(b, i) {
                    let v = eval_subst(vm, &s[i + 1..end])?;
                    out.push_str(&v.to_str());
                    i = end + 1;
                } else {
                    out.push('[');
                    i += 1;
                }
            }
            b'$' if variables && i + 1 < n => {
                if let Some((name, next)) = parse_var_ref(s, i) {
                    if let Err(c) = vm.fire_var_traces(name, "read") {
                        return Err(TclError::new(c.result.to_str().to_string()));
                    }
                    let v = vm.var_get(name).ok_or_else(|| {
                        TclError::new(format!("can't read \"{name}\": no such variable"))
                    })?;
                    out.push_str(&v.to_str());
                    i = next;
                } else {
                    out.push('$');
                    i += 1;
                }
            }
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

/// Parse a `$`-variable reference starting at `s[at]` (`$name`, `${name}`,
/// `$name(idx)`), returning `(name, index_past_reference)`.
fn parse_var_ref(s: &str, at: usize) -> Option<(&str, usize)> {
    let b = s.as_bytes();
    let n = b.len();
    if b.get(at + 1) == Some(&b'{') {
        let rel = s[at + 2..].find('}')?;
        let close = at + 2 + rel;
        return Some((&s[at + 2..close], close + 1));
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
        return Some((&s[start..=j + rel], j + rel + 1));
    }
    Some((&s[start..j], j))
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
    let mut out = String::with_capacity(n);
    let mut i = 0usize;
    let mut lit = 0usize;
    while i < n {
        match b[i] {
            // `\X` is a literal escape, not a trigger; skip it in the scan (it
            // is decoded with the surrounding literal run when copied).
            b'\\' => i = (i + 2).min(n),
            b'$' if i + 1 < n && b[i + 1] == b'{' => {
                out.push_str(&tcl_syntax::backslash::decode(&word[lit..i]));
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
                    out.push_str(&tcl_syntax::backslash::decode(&word[lit..i]));
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
    out.push_str(&tcl_syntax::backslash::decode(&word[lit..n]));
    Ok(Value::string(out))
}

#[cfg(test)]
mod tests {
    use super::{command_end, parse_var_ref, whole_braced};

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

    #[test]
    fn parse_var_ref_handles_plain_braced_array_and_namespace() {
        // `$name` — bare alphanumeric run.
        assert_eq!(parse_var_ref("$foo", 0), Some(("foo", 4)));
        // `${name}` — braces allow spaces and end the reference at `}`.
        assert_eq!(parse_var_ref("${foo bar}", 0), Some(("foo bar", 10)));
        // `$name(idx)` — the array index is part of the captured name.
        assert_eq!(parse_var_ref("$foo(1)", 0), Some(("foo(1)", 7)));
        // `$a::b` — namespace separators extend the name.
        assert_eq!(parse_var_ref("$a::b", 0), Some(("a::b", 5)));
    }

    #[test]
    fn parse_var_ref_rejects_a_bare_dollar() {
        // `$` at end of string, or `$` not followed by a name char.
        assert_eq!(parse_var_ref("$", 0), None);
        assert_eq!(parse_var_ref("$ x", 0), None);
    }
}
