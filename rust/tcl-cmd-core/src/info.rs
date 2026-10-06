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

//! `info` subcommand cores — the *stateful* (Family-B) half of `info`, written
//! once over a runtime's [`Introspect`] role trait + [`ValueOps`].
//!
//! Unlike the pure value cores in this crate, these reach into runtime state
//! (the call-stack), so they are generic over a type that implements **both**
//! `ValueOps` (to read the argument and build the result) and the matching
//! Family-B role trait. Both runtimes (`tcl-vm`, `runtime/rust`) satisfy the
//! bound and wrap the `Result<V, CmdError>` in their own command ABI.

use tcl_runtime_api::{Frames, Introspect, Namespaces, NsId, Procs, ROOT_NS, VarStore};
use tcl_syntax::glob::{is_literal_bytes, string_match_bytes};
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `info complete script` — whether `script` is a syntactically complete command
/// (or sequence): no unclosed `{}`/`[]`/`"` and no trailing backslash
/// continuation. Mirrors C's `Tcl_CommandComplete`.
///
/// Pure (no runtime state), unlike the rest of this module. Crucially, command
/// substitution is **not** parsed inside `{braces}` (a `[` there is literal), so
/// `{[}` is complete, where a naive bracket counter would call it incomplete.
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
    Err(CmdError::new(format!(
        "bad level \"{}\"",
        ops.try_as_str(n)?
    )))
}

/// `info exists varName` — whether `varName` is currently set in the current
/// scope: a scalar, an array, or an array element (`a(k)`). Mirrors
/// `Tcl_InfoExistsCmd` — the existence check resolves the name exactly as a read
/// would, through [`VarStore::exists`] against the current frame.
pub fn exists<O, V>(ops: &mut O, name: &V) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.native_string_bytes(name)?;
    let present = ops.exists_bytes(here, &name)?;
    Ok(ops.new_bool(present))
}

/// `info body procname` — the source body of procedure `name`. Errors with
/// `"name" isn't a procedure` when `name` does not name a user proc. Mirrors
/// C's `InfoBodyCmd`.
pub fn body<O, V>(ops: &mut O, name: &V) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Procs,
{
    let n = ops.native_string_bytes(name)?;
    match ops.proc_body_bytes(&n)? {
        Some(body) => Ok(ops.new_bytes(&body)),
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
    let n = ops.native_string_bytes(name)?;
    let Some(parameters) = ops.proc_formal_names_bytes(&n)? else {
        return Err(not_a_proc(&n));
    };
    let mut names = Vec::with_capacity(parameters.len());
    for parameter in &parameters {
        let name = ops.formal_introspection_name_bytes(parameter)?;
        names.push(ops.new_bytes(&name));
    }
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
    let n = ops.native_string_bytes(name)?;
    let a = ops.native_string_bytes(arg)?;
    match ops.proc_default_value_bytes(&n, &a)? {
        tcl_runtime_api::ProcDefaultValue::MissingProcedure => Err(not_a_proc(&n)),
        tcl_runtime_api::ProcDefaultValue::MissingParameter => {
            let mut message = b"procedure \"".to_vec();
            message.extend_from_slice(&n);
            message.extend_from_slice(b"\" doesn't have an argument \"");
            message.extend_from_slice(&a);
            message.push(b'"');
            Err(CmdError::new_bytes(message))
        }
        tcl_runtime_api::ProcDefaultValue::Declared(Some(value)) => Ok((value, true)),
        tcl_runtime_api::ProcDefaultValue::Declared(None) => Ok((ops.new_bytes(b""), false)),
    }
}

/// `"name" isn't a procedure` — the shared error the proc-introspection
/// subcommands (`info body`/`args`/`default`) raise for a non-proc target.
fn not_a_proc(name: &[u8]) -> CmdError {
    let mut message = vec![b'"'];
    message.extend_from_slice(name);
    message.extend_from_slice(b"\" isn't a procedure");
    CmdError::new_bytes(message)
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
///   back to global) — `info procs` never merges global. The asymmetry is C's.
///
/// Results are sorted, so the listing is deterministic rather than following
/// C's hash order.
pub fn command_list<O, V>(ops: &mut O, pattern: Option<&V>, procs_only: bool) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Namespaces,
{
    let pat = pattern
        .map(|value| ops.native_string_bytes(value).map(|bytes| bytes.to_vec()))
        .transpose()?;
    let policy = ops.name_policy_protocol().ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "command enumeration name issuer",
        ),
    )?;
    if policy.recipe().is_jim084() {
        let current = Namespaces::current(ops);
        let rooted = Namespaces::name_bytes(ops, current);
        let namespace = rooted.strip_prefix(b"::").ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim command enumeration namespace object",
            ),
        )?;
        let candidates = if procs_only {
            ops.procs_in_bytes(ROOT_NS)
        } else {
            ops.commands_in_bytes(ROOT_NS)
        };
        let names = policy
            .recipe()
            .jim_info_command_names(namespace, pat.as_deref(), &candidates, procs_only)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "Jim command enumeration projection",
                )
            })?;
        return Ok(build_name_list_bytes(ops, names));
    }
    let cur = Namespaces::current(ops);
    let names = if let Some((prefix, tail)) = pat.as_deref().and_then(split_last_qualifier_bytes) {
        qualified_listing_bytes(ops, prefix, tail, cur, |o, id| {
            if procs_only {
                o.procs_in_bytes(id)
            } else {
                o.commands_in_bytes(id)
            }
        })?
    } else {
        let mut v = if procs_only {
            ops.procs_in_bytes(cur)
        } else {
            ops.commands_in_bytes(cur)
        };
        // `info commands` (not `info procs`) also sees the global commands.
        if !procs_only && cur != ROOT_NS {
            v.extend(ops.commands_in_bytes(ROOT_NS));
        }
        finish_unqualified_bytes(v, pat.as_deref())
    };
    Ok(build_name_list_bytes(ops, names))
}

fn jim_namespace_variables<O, V>(ops: &mut O, pattern: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Namespaces,
{
    let pattern = pattern
        .map(|pattern| ops.native_string_bytes(pattern).map(|bytes| bytes.to_vec()))
        .transpose()?
        .unwrap_or_else(|| b"*".to_vec());
    let rooted = pattern.starts_with(b"::");
    let current = Namespaces::current(ops);
    let namespace = Namespaces::name_bytes(ops, current);
    let key = tcl_syntax::naming::jim_global_variable_key_bytes(&namespace, &pattern);
    let mut names = filter_ordered_names(ops.vars_in_bytes_checked(ROOT_NS)?, Some(&key));
    if rooted {
        names = names
            .into_iter()
            .map(|name| {
                let mut rooted = b"::".to_vec();
                rooted.extend_from_slice(&name);
                rooted
            })
            .collect();
    }
    Ok(build_name_list_bytes(ops, names))
}

/// `info vars ?pattern?` — the variables visible in the current context (C's
/// `InfoVarsCmd`):
///
/// - A **namespace-qualified** `pattern` lists that namespace's variables,
///   re-qualified absolute (exactly as [`command_list`] does for commands).
/// - **In a procedure**, an unqualified pattern lists the frame's own variables —
///   genuine locals *and* `upvar`/`global`/`variable` links (by their local name).
/// - **At namespace/global scope**, it lists the current namespace's variables.
pub fn vars<O, V>(ops: &mut O, pattern: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Namespaces + Frames,
{
    if Namespaces::variable_lookup_policy(ops) == Some(tcl_dialect::VariableLookupPolicy::Jim) {
        return jim_namespace_variables(ops, pattern);
    }
    let pat = pattern
        .map(|value| ops.native_string_bytes(value).map(|bytes| bytes.to_vec()))
        .transpose()?;
    let cur = Namespaces::current(ops);
    let names = if let Some((prefix, tail)) = pat.as_deref().and_then(split_last_qualifier_bytes) {
        qualified_variable_listing_bytes(ops, prefix, tail, cur)?
    } else if Frames::in_proc(ops) {
        filter_ordered_names(ops.var_names_bytes_checked(true)?, pat.as_deref())
    } else {
        filter_ordered_names(ops.vars_in_bytes_checked(cur)?, pat.as_deref())
    };
    Ok(build_name_list_bytes(ops, names))
}

/// `info locals ?pattern?` — the genuine local variables (no `upvar`/`global`/
/// `variable` links) of the current procedure frame; empty outside a proc.
pub fn locals<O, V>(ops: &mut O, pattern: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Frames,
{
    let pat = pattern
        .map(|value| ops.native_string_bytes(value).map(|bytes| bytes.to_vec()))
        .transpose()?;
    let names = filter_ordered_names(ops.var_names_bytes_checked(false)?, pat.as_deref());
    Ok(build_name_list_bytes(ops, names))
}

/// `info globals ?pattern?` — the variables of the global namespace. A
/// `::`-prefixed pattern matches global variables written absolute (Bug 1057461:
/// strip *all* leading colons, so `::x`/`:::x` match `x`, but a lone `:x` does not).
pub fn globals<O, V>(ops: &mut O, pattern: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Namespaces,
{
    if Namespaces::variable_lookup_policy(ops) == Some(tcl_dialect::VariableLookupPolicy::Jim) {
        return jim_namespace_variables(ops, pattern);
    }
    let pat = pattern
        .map(|value| {
            let bytes = ops.native_string_bytes(value)?;
            let selected = if bytes.starts_with(b"::") {
                bytes
                    .iter()
                    .position(|byte| *byte != b':')
                    .map_or(&[][..], |start| &bytes[start..])
            } else {
                bytes.as_ref()
            };
            Ok::<_, tcl_syntax::value::ValueError>(selected.to_vec())
        })
        .transpose()?;
    let names = filter_ordered_names(ops.vars_in_bytes_checked(ROOT_NS)?, pat.as_deref());
    Ok(build_name_list_bytes(ops, names))
}

/// `info consts ?pattern?` — enumerate constant **bindings**, not variables
/// reached through links. Qualified patterns select exactly one namespace;
/// a procedure lists its direct local bindings; namespace scope also exposes
/// unshadowed global constants, matching Tcl's `InfoConstsCmd`.
///
/// This path stays byte-valued through enumeration, pattern matching, and
/// result construction so a byte-native runtime does not corrupt a Tcl name.
pub fn consts<O, V>(ops: &mut O, pattern: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Namespaces + Frames,
{
    let pat = pattern
        .map(|p| ops.native_string_bytes(p).map(|bytes| bytes.to_vec()))
        .transpose()?;
    let cur = Namespaces::current(ops);
    let names = if let Some((prefix, tail)) = pat.as_deref().and_then(split_last_qualifier_bytes) {
        qualified_listing_bytes(ops, prefix, tail, cur, Namespaces::consts_in_bytes)?
    } else {
        let mut names = if Frames::in_proc(ops) {
            ops.const_names_bytes()
        } else {
            let mut current = ops.consts_in_bytes(cur);
            if cur != ROOT_NS {
                let global = ops.consts_in_bytes(ROOT_NS);
                if let Some(exact) = pat.as_deref().filter(|pat| is_literal_bytes(pat)) {
                    // `TclInfoConstsCmd` preserves its direct-lookup fast path
                    // as observable behaviour: a non-constant local binding
                    // hides a global constant from a glob scan, but an exact
                    // unqualified lookup falls through to that global.
                    current = current
                        .into_iter()
                        .find(|name| name.as_slice() == exact)
                        .or_else(|| global.into_iter().find(|name| name.as_slice() == exact))
                        .into_iter()
                        .collect();
                } else {
                    let shadowed = ops.vars_in_bytes(cur);
                    current.extend(global.into_iter().filter(|name| !shadowed.contains(name)));
                }
            }
            current
        };
        names.sort();
        names.dedup();
        if let Some(pattern) = pat.as_deref() {
            names.retain(|name| string_match_bytes(pattern, name));
        }
        names
    };
    Ok(build_name_list_bytes(ops, names))
}

fn qualified_listing_bytes<O, F>(
    ops: &O,
    prefix: &[u8],
    tail: &[u8],
    cur: NsId,
    enumerate: F,
) -> Result<Vec<Vec<u8>>, CmdError>
where
    O: Namespaces,
    F: Fn(&O, NsId) -> Vec<Vec<u8>>,
{
    let target = if prefix.is_empty() {
        Some(ROOT_NS)
    } else {
        ops.find_namespace_bytes_checked(cur, prefix)?
    };
    let Some(id) = target else {
        return Ok(Vec::new());
    };
    let mut raw = enumerate(ops, id);
    raw.sort();
    let mut prefix_bytes = ops.name_bytes(id);
    if id != ROOT_NS {
        prefix_bytes.extend_from_slice(b"::");
    }
    Ok(raw
        .into_iter()
        .filter(|name| string_match_bytes(tail, name))
        .map(|name| {
            let mut full_name = prefix_bytes.clone();
            full_name.extend_from_slice(&name);
            full_name
        })
        .collect())
}

fn qualified_variable_listing_bytes<O: Namespaces>(
    ops: &O,
    prefix: &[u8],
    tail: &[u8],
    current: NsId,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let target = if prefix.is_empty() {
        Some(ROOT_NS)
    } else {
        ops.find_namespace_bytes_checked(current, prefix)?
    };
    let Some(target) = target else {
        return Ok(Vec::new());
    };
    let names = ops.vars_in_bytes_checked(target)?;
    let mut prefix = ops.name_bytes(target);
    if target != ROOT_NS {
        prefix.extend_from_slice(b"::");
    }
    Ok(filter_ordered_names(names, Some(tail))
        .into_iter()
        .map(|name| {
            let mut full = prefix.clone();
            full.extend_from_slice(&name);
            full
        })
        .collect())
}

/// Filter an already ordered native inventory without changing entry identity,
/// declaration multiplicity, or physical table traversal.
fn filter_ordered_names(mut names: Vec<Vec<u8>>, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
    if let Some(pattern) = pattern {
        names.retain(|name| string_match_bytes(pattern, name));
    }
    names
}

/// Sort, dedupe, and glob-filter `names` by `pat` — the unqualified-listing tail
/// shared by the `info` listing cores. Sorting makes the listing deterministic
/// rather than following C's hash order.
fn finish_unqualified_bytes(mut names: Vec<Vec<u8>>, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
    names.sort();
    names.dedup();
    if let Some(pattern) = pattern {
        names.retain(|name| string_match_bytes(pattern, name));
    }
    names
}

fn build_name_list_bytes<O, V>(ops: &mut O, names: Vec<Vec<u8>>) -> V
where
    O: ValueOps<Value = V>,
{
    let vals: Vec<V> = names.into_iter().map(|name| ops.new_bytes(&name)).collect();
    ops.new_list(vals)
}

/// Split a listing pattern on its **last** `::` into `(ns_prefix, tail_glob)`, or
/// `None` when the pattern is unqualified. An empty prefix (a leading `::pat`)
/// denotes the global namespace. Matches C's `TclGetNamespaceForQualName` split
/// on colon runs (`foo:::bar` → prefix `foo`, tail `bar`).
fn split_last_qualifier_bytes(p: &[u8]) -> Option<(&[u8], &[u8])> {
    use crate::namespace::Qualifier;

    match crate::namespace::qualifier(p) {
        Qualifier::Absolute(b"::") => Some((b"", crate::namespace::tail(p))),
        Qualifier::Absolute(prefix) | Qualifier::Relative(prefix) => {
            Some((prefix, crate::namespace::tail(p)))
        }
        Qualifier::Unqualified => None,
    }
}

#[cfg(test)]
mod tests {
    use super::complete;

    #[test]
    fn native_variable_filter_preserves_order_and_duplicate_declarations() {
        let names = [
            b"x".to_vec(),
            b"x".to_vec(),
            b"k05".to_vec(),
            b"k00".to_vec(),
        ];
        assert_eq!(super::filter_ordered_names(names.to_vec(), None), names);
        assert_eq!(
            super::filter_ordered_names(names.to_vec(), Some(b"x")),
            [b"x".to_vec(), b"x".to_vec()]
        );
    }

    #[test]
    fn complete_matches_c_semantics() {
        assert!(complete(b"set x 1"));
        assert!(!complete(b"set x {")); // unclosed brace
        assert!(!complete(b"set x [")); // unclosed bracket
        assert!(!complete(b"a \"")); // unclosed quote
        assert!(!complete(b"a \\")); // trailing backslash continuation
        // `[` is literal inside braces, so `{[}` is complete — a plain bracket
        // counter would report it incomplete.
        assert!(complete(b"{[}"));
        assert!(complete(b"puts {a [ b}"));
    }
}
