//! `info` subcommand cores — the *stateful* (Family-B) half of `info`, written
//! once over a runtime's [`Introspect`] role trait + [`ValueOps`].
//!
//! Unlike the pure value cores in this crate, these reach into runtime state
//! (the call-stack), so they are generic over a type that implements **both**
//! `ValueOps` (to read the argument and build the result) and the matching
//! Family-B role trait. Both runtimes (`tcl-vm`, `runtime/rust`) satisfy the
//! bound and wrap the `Result<V, CmdError>` in their own command ABI.

use tcl_runtime_api::{Frames, Introspect, Namespaces, NsId, Procs, ROOT_NS, VarStore};
use tcl_syntax::glob::string_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `info complete script` — whether `script` is a syntactically complete command
/// (or sequence): no unclosed `{}`/`[]`/`"` and no trailing backslash
/// continuation. Mirrors C's `Tcl_CommandComplete`.
///
/// Pure (no runtime state), unlike the rest of this module. Crucially, command
/// substitution is **not** parsed inside `{braces}` (a `[` there is literal), so
/// `{[}` is complete — where a naive bracket counter (the VM's old `is_complete`)
/// diverged.
#[must_use]
pub fn complete(s: &[u8]) -> bool {
    let mut stack: Vec<u8> = Vec::new(); // expected closers: `}` or `]`
    let mut in_quote = false;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'\\' {
            // A backslash escapes the next byte; a trailing one leaves the
            // command incomplete (line continuation awaiting more input).
            if i + 1 >= s.len() {
                return false;
            }
            i += 2;
            continue;
        }
        let in_brace = stack.last() == Some(&b'}');
        if in_brace {
            // Inside braces only brace nesting matters — `[`/`"` are literal.
            match c {
                b'{' => stack.push(b'}'),
                b'}' => {
                    stack.pop();
                }
                _ => {}
            }
        } else if in_quote {
            match c {
                b'"' => in_quote = false,
                b'[' => stack.push(b']'),
                b']' if stack.last() == Some(&b']') => {
                    stack.pop();
                }
                _ => {}
            }
        } else {
            match c {
                b'{' => stack.push(b'}'),
                b'[' => stack.push(b']'),
                b']' => {
                    if stack.last() == Some(&b']') {
                        stack.pop();
                    }
                }
                b'"' => in_quote = true,
                _ => {}
            }
        }
        i += 1;
    }
    stack.is_empty() && !in_quote
}

/// `info level ?number?` — with no argument, the current call-stack depth; with
/// `number`, the command words (proc name + args) of that level. A positive
/// `number` is an absolute level; zero or negative is relative to the current
/// level (`info level 0` is the current procedure's own invocation). Mirrors
/// `Tcl_InfoLevel` / `tclCmdIL.c` `InfoLevelCmd`.
///
/// A non-integer `number` is the standard coercion error (`expected integer but
/// got "x"`); a level outside `1..=current` is `bad level "x"`.
pub fn level<O, V>(ops: &mut O, number: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Introspect<Value = V>,
{
    let cur = i64::try_from(ops.level()).unwrap_or(i64::MAX);
    let Some(n) = number else {
        return Ok(ops.new_int(cur));
    };
    // Non-integer → `expected integer but got "x"` (via `ValueError`).
    let requested = ops.as_int(n)?;
    let target = if requested <= 0 {
        cur + requested
    } else {
        requested
    };
    if target >= 1
        && let Some(argv) = ops.level_argv(usize::try_from(target).unwrap_or(0))
    {
        return Ok(argv);
    }
    // Syntactically an integer but no such call frame (out of range, or <= 0
    // at the global level): `bad level "x"`.
    Err(CmdError::new(format!("bad level \"{}\"", ops.as_str(n))))
}

/// `info exists varName` — whether `varName` is currently set in the current
/// scope: a scalar, an array, or an array element (`a(k)`). Mirrors
/// `Tcl_InfoExistsCmd` — the existence check resolves the name exactly as a read
/// would, through [`VarStore::exists`] against the current frame.
pub fn exists<O, V>(ops: &mut O, name: &V) -> V
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.as_str(name);
    let present = ops.exists(here, &name);
    ops.new_bool(present)
}

/// `info body procname` — the source body of procedure `name`. Errors with
/// `"name" isn't a procedure` when `name` does not name a user proc. Mirrors
/// C's `InfoBodyCmd`.
pub fn body<O, V>(ops: &mut O, name: &V) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Procs,
{
    let n = ops.as_str(name);
    match ops.proc_info(&n) {
        Some(info) => Ok(ops.new_bytes(&info.body)),
        None => Err(not_a_proc(&n)),
    }
}

/// `info args procname` — the formal parameter names of procedure `name`, in
/// declaration order (not sorted). Errors as [`body`] does for a non-proc.
/// Mirrors C's `InfoArgsCmd`.
pub fn args<O, V>(ops: &mut O, name: &V) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Procs,
{
    let n = ops.as_str(name);
    let Some(info) = ops.proc_info(&n) else {
        return Err(not_a_proc(&n));
    };
    let names: Vec<V> = info.params.iter().map(|p| ops.new_bytes(&p.name)).collect();
    Ok(ops.new_list(names))
}

/// `info default procname arg varname` — the default value of formal parameter
/// `arg` of procedure `name`. Returns the `(value, has_default)` pair: the
/// declared default (or the empty string when the parameter has none) and
/// whether a default was declared. The caller writes `value` into `varname`
/// (a write trace or array-typed target can fail, so the store stays in the
/// adapter) and returns `has_default` as the result — C's `InfoDefaultCmd`
/// returns 1/0 accordingly.
///
/// Errors `"name" isn't a procedure` for a non-proc, or `procedure "name"
/// doesn't have an argument "arg"` when the parameter is unknown.
pub fn default<O, V>(ops: &mut O, name: &V, arg: &V) -> Result<(V, bool), CmdError>
where
    O: ValueOps<Value = V> + Procs,
{
    let n = ops.as_str(name);
    let a = ops.as_str(arg);
    let Some(info) = ops.proc_info(&n) else {
        return Err(not_a_proc(&n));
    };
    let Some(param) = info
        .params
        .iter()
        .find(|p| p.name.as_slice() == a.as_bytes())
    else {
        return Err(CmdError::new(format!(
            "procedure \"{n}\" doesn't have an argument \"{a}\""
        )));
    };
    let (bytes, has): (&[u8], bool) = match &param.default {
        Some(d) => (d, true),
        None => (&[], false),
    };
    Ok((ops.new_bytes(bytes), has))
}

/// `"name" isn't a procedure` — the shared error the proc-introspection
/// subcommands (`info body`/`args`/`default`) raise for a non-proc target.
fn not_a_proc(name: &str) -> CmdError {
    CmdError::new(format!("\"{name}\" isn't a procedure"))
}

/// `info commands ?pattern?` (`procs_only == false`) / `info procs ?pattern?`
/// (`procs_only == true`) — the names of commands (or just user procedures)
/// reachable by the given pattern, mirroring C's `InfoCommandsCmd`/`InfoProcsCmd`
/// namespace handling:
///
/// - A **namespace-qualified** `pattern` (`::ns::glob` or `ns::glob`) lists the
///   matching members of *that* namespace only, re-qualified to absolute names
///   (`::ns::name`). The qualifier resolves relative to the current namespace; an
///   unknown namespace yields the empty list.
/// - An **unqualified** `pattern` (or none) lists the current namespace's members
///   glob-filtered. For `info commands` only, the global namespace's commands are
///   merged in when the current namespace is not global (command resolution falls
///   back to global) — `info procs` never merges global (the C asymmetry: the
///   runtime's old `visible_proc_names` wrongly merged it).
///
/// Results are sorted (deterministic, unlike C's hash order — matching both
/// runtimes' prior behaviour).
pub fn command_list<O, V>(ops: &mut O, pattern: Option<&V>, procs_only: bool) -> V
where
    O: ValueOps<Value = V> + Namespaces,
{
    let pat: Option<String> = pattern.map(|p| ops.as_str(p).to_string());
    let cur = Namespaces::current(ops);
    let names = if let Some((prefix, tail)) = pat.as_deref().and_then(split_last_qualifier) {
        qualified_listing(ops, prefix, tail, cur, |o, id| {
            if procs_only {
                o.procs_in(id)
            } else {
                o.commands_in(id)
            }
        })
    } else {
        let mut v = if procs_only {
            ops.procs_in(cur)
        } else {
            ops.commands_in(cur)
        };
        // `info commands` (not `info procs`) also sees the global commands.
        if !procs_only && cur != ROOT_NS {
            v.extend(ops.commands_in(ROOT_NS));
        }
        finish_unqualified(v, pat.as_deref())
    };
    build_name_list(ops, names)
}

/// `info vars ?pattern?` — the variables visible in the current context (C's
/// `InfoVarsCmd`):
///
/// - A **namespace-qualified** `pattern` lists that namespace's variables,
///   re-qualified absolute (exactly as [`command_list`] does for commands).
/// - **In a procedure**, an unqualified pattern lists the frame's own variables —
///   genuine locals *and* `upvar`/`global`/`variable` links (by their local name).
/// - **At namespace/global scope**, it lists the current namespace's variables.
pub fn vars<O, V>(ops: &mut O, pattern: Option<&V>) -> V
where
    O: ValueOps<Value = V> + Namespaces + Frames,
{
    let pat: Option<String> = pattern.map(|p| ops.as_str(p).to_string());
    let cur = Namespaces::current(ops);
    let names = if let Some((prefix, tail)) = pat.as_deref().and_then(split_last_qualifier) {
        qualified_listing(ops, prefix, tail, cur, Namespaces::vars_in)
    } else if Frames::in_proc(ops) {
        finish_unqualified(ops.var_names(true), pat.as_deref())
    } else {
        finish_unqualified(ops.vars_in(cur), pat.as_deref())
    };
    build_name_list(ops, names)
}

/// `info locals ?pattern?` — the genuine local variables (no `upvar`/`global`/
/// `variable` links) of the current procedure frame; empty outside a proc.
pub fn locals<O, V>(ops: &mut O, pattern: Option<&V>) -> V
where
    O: ValueOps<Value = V> + Frames,
{
    let pat: Option<String> = pattern.map(|p| ops.as_str(p).to_string());
    let names = finish_unqualified(ops.var_names(false), pat.as_deref());
    build_name_list(ops, names)
}

/// `info globals ?pattern?` — the variables of the global namespace. A
/// `::`-prefixed pattern matches global variables written absolute (Bug 1057461:
/// strip *all* leading colons, so `::x`/`:::x` match `x`, but a lone `:x` does not).
pub fn globals<O, V>(ops: &mut O, pattern: Option<&V>) -> V
where
    O: ValueOps<Value = V> + Namespaces,
{
    let pat: Option<String> = pattern.map(|p| {
        let s = ops.as_str(p);
        if s.starts_with("::") {
            s.trim_start_matches(':').to_string()
        } else {
            s.to_string()
        }
    });
    let names = finish_unqualified(ops.vars_in(ROOT_NS), pat.as_deref());
    build_name_list(ops, names)
}

/// The qualified-pattern listing path shared by `info commands`/`procs`/`vars`:
/// resolve `prefix` (relative to `cur`, or the global root if empty) to a
/// namespace, enumerate its members via `enumerate`, re-qualify each to an
/// absolute name, and glob-filter by `tail`. An unknown namespace → empty.
fn qualified_listing<O, F>(
    ops: &O,
    prefix: &str,
    tail: &str,
    cur: NsId,
    enumerate: F,
) -> Vec<String>
where
    O: Namespaces,
    F: Fn(&O, NsId) -> Vec<String>,
{
    let target = if prefix.is_empty() {
        Some(ROOT_NS)
    } else {
        ops.find_namespace(cur, prefix)
    };
    let Some(id) = target else {
        return Vec::new();
    };
    let mut raw = enumerate(ops, id);
    raw.sort();
    let canon = ops.name(id); // "::" for root, "::foo" otherwise
    let qual = if id == ROOT_NS {
        canon
    } else {
        format!("{canon}::")
    };
    raw.into_iter()
        .filter(|n| string_match(tail, n))
        .map(|n| format!("{qual}{n}"))
        .collect()
}

/// Sort, dedupe, and glob-filter `names` by `pat` — the unqualified-listing tail
/// shared by the `info` listing cores. Sorting is deterministic (unlike C's hash
/// order — matching both runtimes' prior behaviour).
fn finish_unqualified(mut names: Vec<String>, pat: Option<&str>) -> Vec<String> {
    names.sort();
    names.dedup();
    if let Some(p) = pat {
        names.retain(|n| string_match(p, n));
    }
    names
}

/// Build a Tcl list value from `names` (each a fresh string value) — the result
/// of every `info` listing core.
fn build_name_list<O, V>(ops: &mut O, names: Vec<String>) -> V
where
    O: ValueOps<Value = V>,
{
    let vals: Vec<V> = names.into_iter().map(|n| ops.new_string(n)).collect();
    ops.new_list(vals)
}

/// Split a listing pattern on its **last** `::` into `(ns_prefix, tail_glob)`, or
/// `None` when the pattern is unqualified. An empty prefix (a leading `::pat`)
/// denotes the global namespace. Matches C's `TclGetNamespaceForQualName` split
/// on colon runs (`foo:::bar` → prefix `foo:`, tail `bar`, like the runtime's
/// `rposition` of `::`).
fn split_last_qualifier(p: &str) -> Option<(&str, &str)> {
    p.rfind("::").map(|i| (&p[..i], &p[i + 2..]))
}

#[cfg(test)]
mod tests {
    use super::complete;

    #[test]
    fn complete_matches_c_semantics() {
        assert!(complete(b"set x 1"));
        assert!(!complete(b"set x {")); // unclosed brace
        assert!(!complete(b"set x [")); // unclosed bracket
        assert!(!complete(b"a \"")); // unclosed quote
        assert!(!complete(b"a \\")); // trailing backslash continuation
        // `[` is literal inside braces — `{[}` is complete (the VM's old bracket
        // counter wrongly reported it incomplete).
        assert!(complete(b"{[}"));
        assert!(complete(b"puts {a [ b}"));
    }
}
