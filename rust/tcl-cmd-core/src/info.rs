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
use tcl_syntax::native_glob::{NativeGlobProtocol, NativeNameGlobPurpose};
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
    if let Some(original) = ops.proc_original_formal_list_value(&n)? {
        return Ok(original);
    }
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
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "Jim enumeration requires original inventory dispatch",
        )
        .into());
    }
    let matcher = NativeGlobProtocol::from_name_policy(policy);
    let pattern = pat.as_deref().map(tcl_core_types::c_string_extent);
    let cur = Namespaces::current(ops);
    let names = if let Some((prefix, tail)) = pattern.and_then(split_last_qualifier_bytes) {
        qualified_listing_bytes(
            ops,
            prefix,
            tail,
            cur,
            |o, id| {
                if procs_only {
                    o.procs_in_bytes(id)
                } else {
                    o.commands_in_bytes(id)
                }
            },
            |pattern, candidate| command_pattern_matches(matcher, pattern, candidate),
        )?
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
        v.sort();
        v.dedup();
        filter_command_names(v, pattern, matcher)?
    };
    // naming.compiler.original-info-commands-literal-resolution
    // C InfoCommandsCmd uses Tcl_GetCommandFullName for qualified matches;
    // the same original windows distinguish its String children from plain
    // unqualified Tcl_NewStringObj table keys. No name bytes donate a cache.
    let qualified = pattern.and_then(split_last_qualifier_bytes).is_some();
    let values = names
        .into_iter()
        .map(|name| {
            if qualified && policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native {
                ops.native_command_full_name_result(&name)
            } else {
                Ok(ops.new_bytes(&name))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ops.new_list(values))
}

/// Direct Jim inventory over an independently retained actual root table.
/// The caller selects the original -nons/namespace-helper boundary and exact
/// -all grammar before entering this operation.
pub fn jim_core_command_list<O, V>(
    ops: &mut O,
    pattern: Option<&V>,
    include_spaces: bool,
    kind: tcl_runtime_api::NativeJimCommandInventoryKind,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Namespaces,
{
    let policy = ops.name_policy_protocol().ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("Jim core inventory issuer"),
    )?;
    if !policy.recipe().is_jim084() {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "Jim core inventory dialect",
        )
        .into());
    }
    let root = ops.root_command_context_checked()?.ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("Jim core inventory root table"),
    )?;
    let candidates = match kind {
        tcl_runtime_api::NativeJimCommandInventoryKind::Commands => ops.commands_in_bytes(root),
        tcl_runtime_api::NativeJimCommandInventoryKind::Procs => ops.procs_in_bytes(root),
        tcl_runtime_api::NativeJimCommandInventoryKind::Aliases => {
            ops.aliases_in_bytes_checked(root)?
        }
    };
    let pattern = pattern
        .map(|value| ops.native_string_bytes(value))
        .transpose()?;
    let names = policy
        .recipe()
        .jim_core_command_names(pattern.as_deref(), &candidates, include_spaces)
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim core inventory native match",
            )
        })?;
    Ok(build_name_list_bytes(ops, names))
}

/// Jim `info alias` returns the actual retained prefix object. Only a reached
/// lookup failure formats the original name; successful queries retain lazy members.
pub fn jim_original_alias<O, V>(ops: &mut O, original_name: &V) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + tcl_runtime_api::Aliases,
{
    let policy = ops.name_policy_protocol().ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("Jim alias introspection issuer"),
    )?;
    if !policy.recipe().is_jim084() {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "Jim alias introspection dialect",
        )
        .into());
    }
    let failure = match ops.alias_prefix_original_value(original_name)? {
        tcl_runtime_api::AliasPrefixLookup::Prefix(original) => return Ok(original),
        tcl_runtime_api::AliasPrefixLookup::MissingCommand => {
            tcl_runtime_api::NativeJimAliasLookupFailure::MissingCommand
        }
        tcl_runtime_api::AliasPrefixLookup::NotAlias => {
            tcl_runtime_api::NativeJimAliasLookupFailure::NotAlias
        }
    };
    let name = ops.native_string_bytes(original_name)?;
    Err(CmdError::new_bytes(failure.message(&name))
        .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084))
}

fn command_pattern_matches(
    matcher: NativeGlobProtocol,
    pattern: &[u8],
    candidate: &[u8],
) -> Result<bool, CmdError> {
    matcher
        .match_name_pattern(
            NativeNameGlobPurpose::InfoCommandsSearch,
            pattern,
            candidate,
        )
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "command enumeration native match",
            )
            .into()
        })
}

fn filter_command_names(
    names: Vec<Vec<u8>>,
    pattern: Option<&[u8]>,
    matcher: NativeGlobProtocol,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let mut selected_names = Vec::new();
    for name in names {
        if pattern.map_or(Ok(true), |pattern| {
            command_pattern_matches(matcher, pattern, &name)
        })? {
            selected_names.push(name);
        }
    }
    Ok(selected_names)
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
    let mut names = filter_ordered_names(
        ops.vars_in_bytes_checked(ROOT_NS)?,
        Some(&key),
        variable_name_matcher(ops)?,
        NativeNameGlobPurpose::InfoVariablesScan,
    )?;
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
    let matcher = variable_name_matcher(ops)?;
    let cur = Namespaces::current(ops);
    let extent = pat.as_deref().map(tcl_core_types::c_string_extent);
    let names = if let Some((prefix, tail)) = extent.and_then(split_last_qualifier_bytes) {
        qualified_variable_listing_bytes(ops, prefix, tail, cur, matcher)?
    } else if Frames::in_proc(ops) {
        filter_frame_names(ops, true, pat.as_deref(), matcher)?
    } else {
        filter_ordered_names(
            ops.vars_in_bytes_checked(cur)?,
            pat.as_deref(),
            matcher,
            NativeNameGlobPurpose::InfoVariablesSearch,
        )?
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
    let names = filter_frame_names(ops, false, pat.as_deref(), variable_name_matcher(ops)?)?;
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
                tcl_core_types::c_string_extent(&bytes)
                    .iter()
                    .position(|byte| *byte != b':')
                    .map_or(&[][..], |start| {
                        &tcl_core_types::c_string_extent(&bytes)[start..]
                    })
            } else {
                bytes.as_ref()
            };
            Ok::<_, tcl_syntax::value::ValueError>(selected.to_vec())
        })
        .transpose()?;
    let names = filter_ordered_names(
        ops.vars_in_bytes_checked(ROOT_NS)?,
        pat.as_deref(),
        variable_name_matcher(ops)?,
        NativeNameGlobPurpose::InfoVariablesSearch,
    )?;
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
        qualified_listing_bytes(
            ops,
            prefix,
            tail,
            cur,
            Namespaces::consts_in_bytes,
            |pattern, candidate| Ok(string_match_bytes(pattern, candidate)),
        )?
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

fn qualified_listing_bytes<O, F, M>(
    ops: &O,
    prefix: &[u8],
    tail: &[u8],
    cur: NsId,
    enumerate: F,
    matches: M,
) -> Result<Vec<Vec<u8>>, CmdError>
where
    O: Namespaces,
    F: Fn(&O, NsId) -> Vec<Vec<u8>>,
    M: Fn(&[u8], &[u8]) -> Result<bool, CmdError>,
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
    let mut names = Vec::new();
    for name in raw {
        if matches(tail, &name)? {
            let mut full_name = prefix_bytes.clone();
            full_name.extend_from_slice(&name);
            names.push(full_name);
        }
    }
    Ok(names)
}

fn qualified_variable_listing_bytes<O: Namespaces>(
    ops: &O,
    prefix: &[u8],
    tail: &[u8],
    current: NsId,
    matcher: NativeGlobProtocol,
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
    Ok(filter_ordered_names(
        names,
        Some(tail),
        matcher,
        NativeNameGlobPurpose::InfoVariablesSearch,
    )?
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
fn filter_frame_names<O: Frames>(
    ops: &O,
    include_links: bool,
    pattern: Option<&[u8]>,
    matcher: NativeGlobProtocol,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let Some(pattern) = pattern else {
        return Ok(ops.var_names_bytes_checked(include_links)?);
    };
    let mut selected = Vec::new();
    for (name, purpose) in ops.var_name_pattern_inputs_bytes_checked(include_links)? {
        if matcher
            .match_name_pattern(purpose, pattern, &name)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original frame variable pattern inputs",
                )
            })?
        {
            selected.push(name);
        }
    }
    Ok(selected)
}

fn variable_name_matcher<O: ValueOps>(ops: &O) -> Result<NativeGlobProtocol, CmdError> {
    ops.name_policy_protocol()
        .map(NativeGlobProtocol::from_name_policy)
        .ok_or_else(|| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable enumeration native pattern purpose",
            )
            .into()
        })
}

fn filter_ordered_names(
    names: Vec<Vec<u8>>,
    pattern: Option<&[u8]>,
    matcher: NativeGlobProtocol,
    purpose: NativeNameGlobPurpose,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let Some(pattern) = pattern else {
        return Ok(names);
    };
    let mut selected = Vec::new();
    for name in names {
        if matcher
            .match_name_pattern(purpose, pattern, &name)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "variable enumeration native pattern purpose",
                )
            })?
        {
            selected.push(name);
        }
    }
    Ok(selected)
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
        let matcher = tcl_syntax::native_glob::NativeGlobProtocol::authored_tcl(
            tcl_dialect::TclVersion::V9_0,
        );
        let purpose = tcl_syntax::native_glob::NativeNameGlobPurpose::InfoVariablesScan;
        assert_eq!(
            super::filter_ordered_names(names.to_vec(), None, matcher, purpose).unwrap(),
            names
        );
        assert_eq!(
            super::filter_ordered_names(names.to_vec(), Some(b"x"), matcher, purpose).unwrap(),
            [b"x".to_vec(), b"x".to_vec()]
        );
    }

    #[test]
    fn variable_name_patterns_keep_original_counted_keys_and_native_scan_units() {
        // Native proof: naming.tcloo.explicit-variable-link-counted-target
        // docs/design/analysis/name-resolution-proofs/explicit-variable-link-counted-target.md
        use tcl_dialect::TclVersion;
        use tcl_syntax::native_glob::{NativeGlobProtocol, NativeNameGlobPurpose};
        let names = [
            b"k\xc0\x80tail".to_vec(),
            b"k\xff".to_vec(),
            b"k\0tail".to_vec(),
        ];
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let matcher = NativeGlobProtocol::authored_tcl(version);
            assert_eq!(
                super::filter_ordered_names(
                    names.to_vec(),
                    Some(b"*"),
                    matcher,
                    NativeNameGlobPurpose::InfoVariablesSearch
                )
                .unwrap(),
                names
            );
            assert_eq!(
                super::filter_ordered_names(
                    names.to_vec(),
                    Some(b"k\0tail"),
                    matcher,
                    NativeNameGlobPurpose::InfoVariablesSearch
                )
                .unwrap(),
                [names[2].clone()]
            );
            assert_eq!(
                super::filter_ordered_names(
                    names.to_vec(),
                    Some(b"k"),
                    matcher,
                    NativeNameGlobPurpose::InfoVariablesSearch
                )
                .unwrap(),
                Vec::<Vec<u8>>::new()
            );
        }
    }

    #[test]
    fn command_name_filter_keeps_native_scan_and_exact_key_purposes() {
        use tcl_dialect::TclVersion;
        use tcl_syntax::native_glob::NativeGlobProtocol;
        let names = [b"raw\xff".to_vec(), b"raw\xc3\xbf".to_vec()];
        for version in TclVersion::ALL {
            let matcher = NativeGlobProtocol::authored_tcl(version);
            assert_eq!(
                super::filter_command_names(names.to_vec(), Some(b"raw?"), matcher).unwrap(),
                names
            );
            assert_eq!(
                super::filter_command_names(names.to_vec(), Some(b"raw[\xff]"), matcher).unwrap(),
                names
            );
            assert_eq!(
                super::filter_command_names(names.to_vec(), Some(b"raw\xff"), matcher).unwrap(),
                [names[0].clone()]
            );
        }
        let matcher = NativeGlobProtocol::from_name_policy(
            tcl_syntax::naming::NamePolicyProtocol::authored_jim084(),
        );
        assert_eq!(
            super::filter_command_names(names.to_vec(), Some(b"raw?"), matcher).unwrap(),
            names
        );
        assert_eq!(
            super::filter_command_names(names.to_vec(), Some(b"raw\xff"), matcher).unwrap(),
            [names[0].clone()]
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
