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
/// Byte length of the UTF-8 character whose leading byte is `first`
/// (defensively 1 for a continuation/invalid byte).
fn utf8_char_len(first: u8) -> usize {
    match first {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        // ASCII (0x00..=0x7F) and continuation / invalid leading bytes.
        _ => 1,
    }
}

/// Length in bytes of the single Tcl backslash escape starting at `b[i]`
/// (which must be `\`). Mirrors reference Tcl's `TclParseBackslash` extent so
/// the substitution loop advances past exactly one escape — including the
/// multi-byte forms (`\xHH…`, `\uHHHH`, `\UHHHHHHHH`, octal `\ooo`, the
/// `\<newline><whitespace>` line continuation) and a `\` before a multi-byte
/// UTF-8 character. (`tcl_syntax::backslash::decode` then decodes that slice.)
#[allow(clippy::many_single_char_names)]
fn backslash_escape_len(b: &[u8], i: usize) -> usize {
    let n = b.len();
    if i + 1 >= n {
        return 1; // trailing backslash → literal `\`
    }
    match b[i + 1] {
        b'x' => {
            // `\x` + every following hex digit (Tcl reads them all).
            let mut j = i + 2;
            while j < n && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j == i + 2 { 2 } else { j - i } // bare `\x` → literal `x`
        }
        b'u' | b'U' => {
            let max = if b[i + 1] == b'u' { 4 } else { 8 };
            let mut j = i + 2;
            let mut k = 0;
            while j < n && k < max && b[j].is_ascii_hexdigit() {
                j += 1;
                k += 1;
            }
            if k == 0 { 2 } else { j - i } // bare `\u`/`\U` → literal letter
        }
        b'0'..=b'7' => {
            // Octal: up to three octal digits (the leading one is b[i+1]).
            let mut j = i + 1;
            let mut k = 0;
            while j < n && k < 3 && (b'0'..=b'7').contains(&b[j]) {
                j += 1;
                k += 1;
            }
            j - i
        }
        b'\n' => {
            // Line continuation: `\`, newline, then leading horizontal space.
            let mut j = i + 2;
            while j < n && (b[j] == b' ' || b[j] == b'\t') {
                j += 1;
            }
            j - i
        }
        other => 1 + utf8_char_len(other),
    }
}

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
                // Decode exactly one backslash escape and advance past it.
                // Slicing a fixed two bytes here split a UTF-8 char boundary
                // when a multi-byte character followed the backslash (panic),
                // and mis-handled the multi-byte escape forms (`\xHH`,
                // `\uHHHH`, `\UHHHHHHHH`, octal, line continuation).
                let len = backslash_escape_len(b, i);
                let decoded = tcl_syntax::backslash::decode(&s[i..i + len]);
                out.push_str(&decoded);
                i += len;
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
