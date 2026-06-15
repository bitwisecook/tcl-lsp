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
    if c.code.is_ok() {
        Ok(c.result)
    } else {
        Err(TclError::new(c.result.to_str().to_string()))
    }
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

    // General scan: copy literal runs, substituting `${…}` and `[…]`.
    let mut out: Vec<u8> = Vec::with_capacity(n);
    let mut i = 0usize;
    let mut lit = 0usize;
    while i < n {
        match b[i] {
            // Keep `\X` literal but stop X from being read as a trigger.
            b'\\' => i = (i + 2).min(n),
            b'$' if i + 1 < n && b[i + 1] == b'{' => {
                out.extend_from_slice(&b[lit..i]);
                if let Some(rel) = word[i + 2..].find('}') {
                    let close = i + 2 + rel;
                    let v = read_var(vm, &word[i + 2..close])?;
                    out.extend_from_slice(v.to_str().as_bytes());
                    i = close + 1;
                } else {
                    out.push(b'$');
                    i += 1;
                }
                lit = i;
            }
            b'[' => {
                if let Some(end) = command_end(b, i) {
                    out.extend_from_slice(&b[lit..i]);
                    let v = eval_subst(vm, &word[i + 1..end])?;
                    out.extend_from_slice(v.to_str().as_bytes());
                    i = end + 1;
                    lit = i;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out.extend_from_slice(&b[lit..n]);
    Ok(Value::string(String::from_utf8_lossy(&out).into_owned()))
}
