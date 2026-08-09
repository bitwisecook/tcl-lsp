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

//! The `namespace` ensemble.
//!
//! `namespace eval ns body` runs `body` with the current namespace set to `ns`
//! so that `proc`/command/variable name resolution qualifies relative to it
//! (see [`Vm::qualify_name`]/[`Vm::lookup_command`]). The introspection
//! subcommands (`current`, `qualifiers`, `tail`, `parent`, `children`,
//! `exists`) operate on canonical names; `export`/`import` are accepted as
//! no-ops for now (the codegen already records export/import metadata).

use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, canonical_cmd_key, err, ok};
use crate::value::Value;

/// Run `body` as a script in namespace `target`, absorbing a top-level
/// `return` at the boundary (a namespace body completes like a proc body).
///
/// The body runs in its own call frame (like a proc) so `info level` counts it
/// and `uplevel`/`upvar` from a proc called within reach it (and its namespace
/// variables). `call_argv` is the invoking command (e.g. `namespace eval ::ns
/// {…}`) for `info level N`.
fn eval_in_ns(vm: &mut Vm, target: String, body: &str, call_argv: Vec<Value>) -> Completion<Value> {
    vm.declare_namespace(&target);
    vm.push_ns_eval_frame(&target, call_argv);
    vm.push_ns(target);
    vm.enter_ns_script();
    let result = vm.eval_source(body);
    vm.leave_ns_script();
    vm.pop_ns();
    vm.pop_call_frame();
    match result {
        Ok(c) if c.code == Code::Return => ok(c.result),
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("namespace", cmd_namespace);
    // The compiled `namespace eval` ensemble rewrite (`invokeReplace … ::tcl::
    // namespace::eval`) dispatches directly to the resolved implementation,
    // dropping the `namespace eval` prefix — so it arrives as
    // `::tcl::namespace::eval ns body …`, exactly `ns_eval`'s argument shape.
    vm.register("::tcl::namespace::eval", ns_eval);
}

/// Display form of a canonical namespace (`""` → `::`, `foo` → `::foo`).
fn display_ns(canonical: &str) -> String {
    if canonical.is_empty() {
        "::".to_string()
    } else {
        format!("::{canonical}")
    }
}

/// Canonicalise a possibly-absolute namespace reference (drop leading `::`),
/// relative names are resolved against the current namespace.
fn canon_ns(vm: &Vm, name: &str) -> String {
    if name == "::" {
        return String::new();
    }
    vm.qualify_name(name)
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn cmd_namespace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"namespace subcommand ?arg ...?\"");
    };
    let sub_word = sub.to_str();
    let profile = tcl_dialect::DialectProfile::by_name(vm.runtime_version().dialect_profile_name());
    let registry = tcl_registry::cache::default_registry();
    let spec = registry
        .get_for_dialect("namespace", profile.availability_mask)
        .expect("the core namespace command is registered for every Tcl release");
    let Some(subcommand) =
        spec.resolve_subcommand_for_dialect(&sub_word, profile.availability_mask)
    else {
        let available: Vec<&str> = spec
            .subcommands
            .iter()
            .filter(|candidate| {
                candidate
                    .dialects
                    .or(spec.dialects)
                    .is_none_or(|gate| gate.intersects(profile.availability_mask))
            })
            .map(|candidate| candidate.name)
            .collect();
        let choices = match available.as_slice() {
            [] => String::new(),
            [only] => (*only).to_string(),
            [head @ .., last] => format!("{}, or {last}", head.join(", ")),
        };
        return err(format!(
            "unknown or ambiguous subcommand \"{sub_word}\": must be {choices}"
        ));
    };
    match subcommand.name {
        "eval" => ns_eval(vm, rest),
        "current" => ok(tcl_cmd_core::namespace::current(vm)),
        "qualifiers" => ns_text_op(rest, tcl_cmd_core::namespace::qualifiers),
        "tail" => ns_text_op(rest, tcl_cmd_core::namespace::tail),
        // exists/parent/children route through the shared core over `Namespaces`
        // (the VM's String model honours the `NsId` handles via its arena). This
        // also gave `children` its missing `?pattern?` filter and made
        // parent/children on a missing namespace error, both matching tclsh.
        "exists" => ok(tcl_cmd_core::namespace::exists(vm, &first(rest))),
        "parent" => {
            let name = rest.first().map(|v| v.to_str().to_string());
            match tcl_cmd_core::namespace::parent(vm, name.as_deref()) {
                Ok(v) => ok(v),
                Err(e) => err(e.into_message()),
            }
        }
        "children" => {
            let name = rest.first().map(|v| v.to_str().to_string());
            let pattern = rest.get(1).map(|v| v.to_str().to_string());
            match tcl_cmd_core::namespace::children(vm, name.as_deref(), pattern.as_deref()) {
                Ok(v) => ok(v),
                Err(e) => err(e.into_message()),
            }
        }
        // `namespace code script` captures the current namespace as a callback
        // command prefix: `::namespace inscope <ns> <script>`.
        "code" => {
            let script = first(rest);
            let ns = display_ns(vm.current_ns());
            ok(Value::list(vec![
                Value::string("::namespace"),
                Value::string("inscope"),
                Value::string(ns),
                Value::string(script),
            ]))
        }
        // `namespace inscope ns script ?arg ...?` runs `script` (with any extra
        // args appended as list elements) in namespace `ns`.
        "inscope" => ns_inscope(vm, rest),
        "which" => {
            // `namespace which ?-command|-variable? name` → the resolved FQN.
            // Default (and `-command`) resolves a command via the shared
            // `Namespaces` core; `-variable` (or an unambiguous prefix) resolves
            // a *variable* and returns its qualified name if it exists, else "".
            // The name is the last word regardless of the flag.
            let want_var = rest
                .first()
                .map(|v| v.to_str().to_string())
                .filter(|_| rest.len() >= 2)
                .is_some_and(|f| f.len() >= 2 && "-variable".starts_with(f.as_str()));
            let name = rest
                .last()
                .map(|v| v.to_str().to_string())
                .unwrap_or_default();
            if want_var {
                if vm.exists_var(&name) {
                    ok(Value::string(display_ns(&vm.qualify_name(&name))))
                } else {
                    ok(Value::empty())
                }
            } else {
                ok(tcl_cmd_core::namespace::which_command(vm, &name))
            }
        }
        "origin" => {
            // `namespace origin command` → the original command's fully-qualified
            // name (following imports). We do not track import provenance, so the
            // resolved qualified name is returned; an unknown command errors.
            let name = first(rest);
            if vm.lookup_command(&name).is_some() {
                ok(Value::string(display_ns(&vm.qualify_name(&name))))
            } else {
                err(format!("invalid command name \"{name}\""))
            }
        }
        "export" => {
            // `namespace export ?-clear? pattern ...` — record export patterns.
            let pats: Vec<String> = rest
                .iter()
                .map(|v| v.to_str().to_string())
                .filter(|p| p != "-clear")
                .collect();
            vm.add_exports(&pats);
            ok(Value::empty())
        }
        "import" => {
            // `namespace import ?-force? pattern ...`
            for p in rest {
                let pat = p.to_str();
                if &*pat == "-force" {
                    continue;
                }
                vm.import_commands(&pat);
            }
            ok(Value::empty())
        }
        // `namespace delete ?ns ...?` — destroy each namespace (and its
        // descendants, commands, and variables). An unknown namespace errors,
        // after deleting any that preceded it (matching tclsh).
        "delete" => {
            for n in rest {
                let canon = canon_ns(vm, &n.to_str());
                if !vm.delete_namespace(&canon) {
                    return err(format!(
                        "unknown namespace \"{}\" in namespace delete command",
                        n.to_str()
                    ));
                }
            }
            ok(Value::empty())
        }
        // `namespace path ?nsList?` — get/set the current namespace's command
        // resolution path. With no list it returns the path as `::`-qualified
        // names; with a list it sets it (entries canonicalised relative to the
        // current namespace). lookup_command consults it (mathop, cmdIL).
        "path" => match rest {
            [] => ok(Value::list(
                vm.ns_path_get()
                    .iter()
                    .map(|n| Value::string(display_ns(n)))
                    .collect(),
            )),
            [list] => {
                let elems = match list.as_list() {
                    Ok(e) => e,
                    Err(e) => return err(e.message),
                };
                let path: Vec<String> = elems.iter().map(|e| canon_ns(vm, &e.to_str())).collect();
                vm.ns_path_set(path);
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"namespace path ?nsList?\""),
        },
        // `namespace forget ?pattern ...?` — remove previously imported commands
        // matching each pattern from the current namespace.
        "forget" => {
            for p in rest {
                if let Err(e) = vm.forget_imports(&p.to_str()) {
                    return err(e);
                }
            }
            ok(Value::empty())
        }
        "ensemble" => ns_ensemble(vm, rest),
        // `namespace unknown ?handler?` (TIP 181) — get/set the CURRENT
        // namespace's resolution-miss handler (a command prefix). Handlers
        // are per-namespace, NOT inherited by children; the global
        // namespace's handler is the interp-wide default; unset reports
        // `::unknown` (the default chain). Consulted by the dispatch miss
        // path before the plain `unknown` proc.
        "unknown" => match rest {
            [] => {
                let h = vm.ns_unknown_get();
                if h.is_empty() {
                    ok(Value::string("::unknown"))
                } else {
                    ok(Value::list(h))
                }
            }
            [handler] => {
                let words = match handler.as_list() {
                    Ok(elems) => elems.to_vec(),
                    Err(e) => return err(e.message),
                };
                vm.ns_unknown_set(words);
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"namespace unknown ?script?\""),
        },
        "upvar" => ns_upvar(vm, rest, subcommand, profile.availability_mask),
        _ => unreachable!("every registry namespace subcommand has VM dispatch"),
    }
}

/// `namespace upvar ns ?otherVar myVar ...?` — link variables in `ns` into
/// the active frame. Subcommand spelling and release availability are resolved
/// from the registry above; this function owns only the storage operation.
fn ns_upvar(
    vm: &mut Vm,
    rest: &[Value],
    subcommand: &tcl_registry::SubCommand,
    dialect: tcl_dialect::DialectSet,
) -> Completion<Value> {
    let argc = u16::try_from(rest.len()).unwrap_or(u16::MAX);
    let valid_arity = subcommand.subcommand_forms.iter().any(|form| {
        form.dialects.is_none_or(|gate| gate.intersects(dialect)) && form.arity.accepts(argc)
    });
    if !valid_arity {
        return err("wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"");
    }

    let namespace_word = rest[0].to_str();
    let namespace = canon_ns(vm, &namespace_word);
    if !vm.namespace_exists(&namespace) {
        return err(format!("namespace \"{namespace_word}\" not found"));
    }

    for pair in rest[1..].chunks_exact(2) {
        let other = pair[0].to_str();
        let local = pair[1].to_str();
        let target = if other.starts_with("::") || namespace.is_empty() {
            canonical_cmd_key(&other).into_owned()
        } else {
            canonical_cmd_key(&format!("::{namespace}::{other}")).into_owned()
        };

        // At namespace scope the alias itself is a namespace variable; inside
        // a proc it is an ordinary local. This is the same frame/link primitive
        // used by `upvar`, so scalars, arrays, byte arrays, and later writes all
        // share one cell rather than copying or stringifying the value.
        if vm.in_ns_script() && !local.contains("::") && !vm.current_ns().is_empty() {
            let alias = vm.qualify_name(&local);
            vm.add_global_link(&alias, 0, &target);
        } else {
            vm.add_link(&local, 0, &target);
        }
    }
    ok(Value::empty())
}

fn first(rest: &[Value]) -> String {
    rest.first()
        .map(|v| v.to_str().to_string())
        .unwrap_or_default()
}

/// `namespace ensemble create|exists|configure` (`tclEnsemble.c`).
fn ns_ensemble(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((op, args)) = rest.split_first() else {
        return err("wrong # args: should be \"namespace ensemble subcommand ?arg ...?\"");
    };
    match &*op.to_str() {
        "create" => ns_ensemble_create(vm, args),
        "exists" => match args {
            [cmd] => ok(Value::bool(matches!(
                vm.lookup_command(&cmd.to_str()),
                Some(crate::command::Command::Ensemble(_))
            ))),
            _ => err("wrong # args: should be \"namespace ensemble exists cmd\""),
        },
        // `configure` query/set is not modelled; accept it so setup code runs.
        "configure" => ok(Value::empty()),
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be configure, create, or exists"
        )),
    }
}

/// `namespace ensemble create ?option value ...?` — build the ensemble command.
fn ns_ensemble_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use crate::command::{Command, EnsembleDef};
    let mut ns = vm.current_ns().to_string();
    let mut command: Option<String> = None;
    let mut map: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
    let mut subcommands: Option<Vec<String>> = None;
    let mut prefixes = true;
    let mut unknown: Option<Vec<Value>> = None;
    let mut i = 0;
    while i < args.len() {
        let opt = args[i].to_str().to_string();
        let Some(val) = args.get(i + 1) else {
            return err(format!("missing value to go with \"{opt}\""));
        };
        match &*opt {
            "-command" => command = Some(val.to_str().to_string()),
            "-namespace" => ns = canon_ns(vm, &val.to_str()),
            "-map" => {
                let elems = match val.as_list() {
                    Ok(e) => e,
                    Err(e) => return err(e.message),
                };
                let mut it = elems.iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    let prefix = v.as_list().map_or_else(|_| vec![v.clone()], |l| l.to_vec());
                    map.insert(k.to_str().to_string(), prefix);
                }
            }
            "-subcommands" => {
                let elems = match val.as_list() {
                    Ok(e) => e,
                    Err(e) => return err(e.message),
                };
                subcommands = Some(elems.iter().map(|v| v.to_str().to_string()).collect());
            }
            "-prefixes" => prefixes = val.as_bool().unwrap_or(true),
            "-unknown" => {
                let elems = match val.as_list() {
                    Ok(e) => e,
                    Err(e) => return err(e.message),
                };
                unknown = (!elems.is_empty()).then(|| elems.to_vec());
            }
            _ => {
                return err(format!(
                    "bad option \"{opt}\": must be -command, -map, -namespace, \
                     -parameters, -prefixes, -subcommands, or -unknown"
                ));
            }
        }
        i += 2;
    }
    // The default command is the namespace itself; an explicit -command binds in
    // the current namespace when unqualified.
    let cmd_key = match command {
        Some(c) => vm.qualify_name(&c),
        None => ns.clone(),
    };
    vm.register_command(
        &cmd_key,
        Command::Ensemble(std::rc::Rc::new(EnsembleDef {
            namespace: ns,
            map,
            subcommands,
            prefixes,
            unknown,
        })),
    );
    ok(Value::string(display_ns(&cmd_key)))
}

/// `namespace qualifiers`/`tail`: run the first argument (lenient — defaults to
/// empty) through a shared `tcl_cmd_core::namespace` text op, as a `Value`. The
/// shared core handles `::`-runs the way C does (the VM's old `rsplit("::")`
/// diverged for 3+ colons, e.g. `tail foo:::`).
fn ns_text_op(rest: &[Value], op: fn(&[u8]) -> &[u8]) -> Completion<Value> {
    let name = first(rest);
    ok(Value::string(
        std::str::from_utf8(op(name.as_bytes())).unwrap_or(""),
    ))
}

fn ns_inscope(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, parts)) = rest.split_first() else {
        return err("wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"");
    };
    let Some((script, extra)) = parts.split_first() else {
        return err("wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"");
    };
    let target = canon_ns(vm, &ns.to_str());
    let body = inscope_script(vm, script, extra);
    let mut call_argv = vec![Value::string("namespace"), Value::string("inscope")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(vm, target, &body, call_argv)
}

/// The script `namespace inscope ns script ?arg ...?` evaluates:
/// `Tcl_ConcatObj(script, list(arg …))` — `NamespaceInscopeCmd`
/// (`generic/tclNamesp.c`) collects the trailing words into a **list object**
/// and concatenates that list's string representation onto `script`. So the
/// tail arrives as list *elements*, not as space-joined script text, and each
/// word reaches the invoked command as exactly one argument however much
/// whitespace or list punctuation it holds:
///
/// ```text
/// namespace inscope :: {puts} {a b}   → prints "a b"  (one argument)
/// namespace eval    :: {puts} {a b}   → error: can not find channel named "a"
/// ```
///
/// (`namespace eval`'s plain space-join is right for *its* concat semantics;
/// `inscope` is the one family member that list-quotes — the registry models
/// the split as `SCRIPT_APPENDS_LIST_ARGS` refining `SCRIPT_CONCATENATES_ARGS`.
/// Issue #1056: the VM used to space-join here too, so `{x y}` became two
/// arguments.)
///
/// Both halves reuse the canonical implementations rather than re-deriving
/// them: the list's string rep comes from `Value::list`'s
/// `tcl_syntax::list::join_list` quoting (`Tcl_ScanElement` /
/// `Tcl_ConvertElement`), and the concatenation itself is the shared
/// `tcl_cmd_core::list::concat` (`Tcl_ConcatObj`) — which trims each part and
/// drops one that is empty after trimming, so an all-whitespace `script`
/// contributes no leading separator.
///
/// With no trailing words C takes the `objc == 3` arm and evaluates `script`
/// verbatim (no concat, hence no trim and no trailing space), which the early
/// return mirrors.
fn inscope_script(vm: &mut Vm, script: &Value, extra: &[Value]) -> String {
    if extra.is_empty() {
        return script.to_str().to_string();
    }
    let tail = Value::list(extra.to_vec());
    tcl_cmd_core::list::concat(vm, &[script.clone(), tail])
        .to_str()
        .to_string()
}

fn ns_eval(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, body_parts)) = rest.split_first() else {
        return err("wrong # args: should be \"namespace eval name arg ?arg ...?\"");
    };
    if body_parts.is_empty() {
        return err("wrong # args: should be \"namespace eval name arg ?arg ...?\"");
    }
    let child = canon_ns(vm, &ns.to_str());
    // Multiple body args are concatenated with spaces, as a script.
    let body = body_parts
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let mut call_argv = vec![Value::string("namespace"), Value::string("eval")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(vm, child, &body, call_argv)
}
