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

use tcl_dialect::model::SurfaceQuery;
use tcl_runtime_api::{Code, Completion};

use crate::command::{command_lookup_error, err_with_code, lookup_error};
use crate::interp::{Vm, canonical_cmd_key, err, ok};
use crate::value::Value;
use tcl_dialect::model::surface_admits;

/// Run `body` as a script in namespace `target`, absorbing a top-level
/// `return` at the boundary (a namespace body completes like a proc body).
///
/// The body runs in its own call frame (like a proc) so `info level` counts it
/// and `uplevel`/`upvar` from a proc called within reach it (and its namespace
/// variables). `call_argv` is the invoking command (e.g. `namespace eval ::ns
/// {…}`) for `info level N`.
fn eval_in_ns(
    vm: &mut Vm,
    written: &str,
    target: String,
    body: &str,
    call_argv: Vec<Value>,
) -> Completion<Value> {
    // A token in its *synchronous* delete window is no longer found by
    // `TclGetNamespaceForQualName`, so `NamespaceEvalCmd` tries to create it —
    // and `Tcl_CreateNamespace` still sees the child entry, which
    // `TclTeardownNamespace` unlinks only after the command loop. tclsh 9.0.4
    // and 8.6.16 both raise `already exists` there. A token whose deletion was
    // *deferred* has already lost that entry, so the same script builds a fresh
    // namespace instead.
    if vm.namespace_is_dying(&target) {
        return err(format!(
            "can't create namespace \"{}\": already exists",
            display_ns(&target)
        ));
    }
    // `TclGetNamespaceForQualName` walks a *relative* name from the frame's own
    // namespace, so from a retained token it reaches that token's retained
    // children; an *absolute* one is rooted at the global namespace, where a
    // recreation lives. tclsh 9.0.4 and 8.6.16 both make the difference
    // visible: from a retained `::N`, `namespace eval C` runs the retained
    // `::N::C` and leaves both names absent, while `namespace eval ::N::C`
    // builds fresh `::N` and `::N::C` that have none of its commands.
    let retained_target = (!written.starts_with("::"))
        .then(|| vm.retained_token_named(&target))
        .flatten();
    if let Some(id) = retained_target {
        vm.push_ns_eval_frame(&target, call_argv);
        vm.push_ns_token(target, id);
    } else {
        vm.declare_namespace(&target);
        vm.push_ns_eval_frame(&target, call_argv);
        vm.push_ns(target);
    }
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
    vm.qualify_namespace_name(name)
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn cmd_namespace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"namespace subcommand ?arg ...?\"");
    };
    let sub_word = sub.to_str();
    // The emulated release's name is a dialect *name*, so it resolves
    // through the one ingress seam (`crate::environment`) and every
    // availability question below is answered under that environment's
    // document authoring mask — one resolution, not a `by_name` here and a
    // mask read at each use (ledger row B1).
    let profile =
        crate::environment::profile_for_dialect(vm.runtime_version().dialect_profile_name());
    let dialect = Some(crate::environment::surface_point(profile));
    let registry = tcl_registry::default_registry();
    let spec = registry
        .get_for_surface("namespace", dialect)
        .expect("the core namespace command is registered for every Tcl release");
    let Some(subcommand) = spec.resolve_subcommand_for_dialect(&sub_word, dialect) else {
        let available: Vec<&str> = spec
            .subcommands
            .iter()
            .filter(|candidate| {
                candidate
                    .surface
                    .or(spec.surface)
                    .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
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
        // During `TclTeardownNamespace` the namespace token is already dead
        // even though its command table stays reachable for delete callbacks.
        // The shared handle lookup intentionally still exposes that table to
        // `info commands`; this lifecycle-aware predicate distinguishes the
        // namespace-existence query.
        "exists" => {
            let canonical = canon_ns(vm, &first(rest));
            ok(Value::bool(vm.namespace_exists(&canonical)))
        }
        "parent" => {
            let name = rest.first().map(|v| v.to_str().to_string());
            match tcl_cmd_core::namespace::parent(vm, name.as_deref()) {
                Ok(v) => ok(v),
                Err(_) => lookup_error(
                    ns_not_found(vm, name.as_deref().unwrap_or_default()),
                    "NAMESPACE",
                    name.as_deref().unwrap_or_default(),
                ),
            }
        }
        "children" => {
            let name = rest.first().map(|v| v.to_str().to_string());
            let pattern = rest.get(1).map(|v| v.to_str().to_string());
            match tcl_cmd_core::namespace::children(vm, name.as_deref(), pattern.as_deref()) {
                Ok(v) => ok(v),
                Err(_) => lookup_error(
                    ns_not_found(vm, name.as_deref().unwrap_or_default()),
                    "NAMESPACE",
                    name.as_deref().unwrap_or_default(),
                ),
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
            let words: Vec<_> = rest.iter().map(|word| word.to_str().to_string()).collect();
            let Some((kind, name_index)) = tcl_cmd_core::namespace::which_request(&words) else {
                return err(
                    "wrong # args: should be \"namespace which ?-command? ?-variable? name\"",
                );
            };
            let name = words[name_index].clone();
            if kind == tcl_cmd_core::namespace::WhichKind::Variable {
                // `Tcl_FindNamespaceVar` semantics via the shared core: the
                // namespace variable tables only, never the call frame (the VM
                // used to gate on `exists_var`, which walks proc locals).
                ok(tcl_cmd_core::namespace::which_variable(vm, &name, profile))
            } else {
                ok(tcl_cmd_core::namespace::which_command(vm, &name))
            }
        }
        "origin" => {
            let argument_count = u16::try_from(rest.len()).unwrap_or(u16::MAX);
            if !subcommand.arity.accepts(argument_count) {
                return err(tcl_cmd_core::CmdError::wrong_args(subcommand.synopsis).into_message());
            }
            // `namespace origin name` → the original command's fully-qualified
            // name (following imports) via the shared `TclGetOriginalCommand`
            // walk.  Visibility is checked before exposing the provenance: an
            // import of a builtin absent from this emulated release is no more
            // observable than a missing command.
            let name = first(rest);
            match tcl_cmd_core::namespace::origin(vm, &name) {
                Some(fqn) if vm.lookup_command(&name).is_some() => ok(Value::string(fqn)),
                _ => command_lookup_error(&name),
            }
        }
        "export" => ns_export(vm, rest),
        "import" => ns_import(vm, rest),
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
                // C resolves every entry with `TclGetNamespaceFromObj` *before*
                // installing the path (`NamespacePathCmd`), so an unresolvable
                // entry errors and leaves the old path in place.
                let mut path: Vec<String> = Vec::with_capacity(elems.len());
                for e in elems.iter() {
                    let written = e.to_str().to_string();
                    let canonical = canon_ns(vm, &written);
                    if !vm.namespace_exists(&canonical) {
                        return err(ns_not_found(vm, &written));
                    }
                    path.push(canonical);
                }
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
                    // Only the global namespace carries a handler by default
                    // (`Tcl_Init` installs `::unknown` there); every other
                    // namespace reports the empty string until one is set —
                    // tclsh 8.6.16/9.0.4-pinned.
                    if vm.current_ns().is_empty() {
                        ok(Value::string("::unknown"))
                    } else {
                        ok(Value::empty())
                    }
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
        "upvar" => ns_upvar(vm, rest, subcommand, dialect),
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
    dialect: Option<SurfaceQuery<'_>>,
) -> Completion<Value> {
    let argc = u16::try_from(rest.len()).unwrap_or(u16::MAX);
    let valid_arity = subcommand.subcommand_forms.iter().any(|form| {
        form.surface
            .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
            && form.arity.accepts(argc)
    });
    if !valid_arity {
        return err("wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"");
    }

    let namespace_word = rest[0].to_str();
    let namespace = canon_ns(vm, &namespace_word);
    if !vm.namespace_exists(&namespace) {
        return err(format!("namespace \"{namespace_word}\" not found"));
    }

    for pair in rest[1..].as_chunks::<2>().0 {
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

/// `TclGetNamespaceFromObj`'s not-found message: a *relative* name names the
/// current namespace context (`… not found in "::ns"`), an absolute one does
/// not (`… not found`).
fn ns_not_found(vm: &Vm, written: &str) -> String {
    if written.starts_with("::") {
        format!("namespace \"{written}\" not found")
    } else {
        format!(
            "namespace \"{written}\" not found in \"{}\"",
            display_ns(vm.current_ns())
        )
    }
}

/// `namespace export ?-clear? ?pattern ...?` (`NamespaceExportCmd` /
/// `Tcl_Export`, `tclNamesp.c:3526-3570`).
///
/// The flag is **positional**: only the first word may be `-clear`, so a
/// second one is an ordinary pattern (`namespace export -clear -clear x`
/// leaves `-clear x` exported — the registry states the same fact as
/// `max_leading_option_words: Some(1)`). With no words at all the command is
/// the query form and reports the current pattern list.
fn ns_export(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    if rest.is_empty() {
        return ok(Value::list(
            vm.exports_get().into_iter().map(Value::string).collect(),
        ));
    }
    let mut words = rest.iter().map(|v| v.to_str().to_string());
    let mut first_word = words.next();
    if first_word.as_deref() == Some("-clear") {
        vm.clear_exports();
        first_word = words.next();
    }
    let patterns: Vec<String> = first_word.into_iter().chain(words).collect();
    // `NamespaceExportCmd` calls `Tcl_Export` once per pattern and returns on
    // the first failure, so the patterns before an invalid one are already
    // committed — validation is NOT a batch gate. (`-clear` is committed
    // earlier still: C spends a whole `Tcl_Export(…, "::", 1)` call on it,
    // which resets the list and then fails its own qualifier check, an error
    // `NamespaceExportCmd` deliberately discards with `Tcl_ResetResult`.)
    // An export pattern names commands in the *current* namespace, so it may
    // not carry a namespace qualifier.
    for pattern in &patterns {
        if tcl_syntax::naming::is_qualified(pattern.as_bytes()) {
            return err(format!(
                "invalid export pattern \"{pattern}\": pattern can't specify a namespace"
            ));
        }
        vm.add_exports(std::slice::from_ref(pattern));
    }
    ok(Value::empty())
}

/// `namespace import ?-force? ?pattern ...?` (`NamespaceImportCmd` /
/// `Tcl_Import`, `tclNamesp.c:3668-3732`).
///
/// `-force` is positional in the same way: only `objv[1]` is read as the flag,
/// and every later word is a pattern — including a trailing `-force`, which
/// then fails the "the pattern must name a source namespace" check.
fn ns_import(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    if rest.is_empty() {
        // The introspection form: the current namespace's imported commands.
        return ok(Value::list(
            vm.imported_command_tails()
                .into_iter()
                .map(Value::string)
                .collect(),
        ));
    }
    let mut words = rest.iter().map(|v| v.to_str().to_string());
    let mut first_word = words.next();
    let allow_overwrite = first_word.as_deref() == Some("-force");
    if allow_overwrite {
        first_word = words.next();
    }
    for pattern in first_word.into_iter().chain(words) {
        let destination = tcl_runtime_api::Namespaces::current(vm);
        if let Err(problem) =
            tcl_cmd_core::namespace::import_pattern(vm, destination, pattern.as_bytes())
        {
            return err(String::from_utf8_lossy(&problem.message()).into_owned());
        }
        if let Err(problem) = vm.import_commands(&pattern, allow_overwrite) {
            return err_with_code(problem.message(), problem.error_code());
        }
    }
    ok(Value::empty())
}

fn first(rest: &[Value]) -> String {
    rest.first()
        .map(|v| v.to_str().to_string())
        .unwrap_or_default()
}

/// `namespace ensemble create|exists|configure` (`TclNamespaceEnsembleCmd`,
/// `tclEnsemble.c:140`). The subcommand word resolves through the shared
/// `ensembleSubcommands` table, so `namespace ensemble cr` is `create` and a
/// miss reads `bad subcommand "…": must be configure, create, or exists`.
fn ns_ensemble(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((op, args)) = rest.split_first() else {
        return err("wrong # args: should be \"namespace ensemble subcommand ?arg ...?\"");
    };
    let index = match tcl_cmd_core::ensemble::SUBCOMMANDS.index_of_str(&op.to_str()) {
        Ok(i) => i,
        Err(e) => return err(e.into_message()),
    };
    match index {
        // configure
        0 => ns_ensemble_configure(vm, args),
        // create
        1 => ns_ensemble_create(vm, args),
        // exists
        _ => match args {
            [cmd] => ok(Value::bool(matches!(
                vm.lookup_command(&cmd.to_str()),
                Some(crate::command::Command::Ensemble(_))
            ))),
            _ => err("wrong # args: should be \"namespace ensemble exists cmdname\""),
        },
    }
}

/// The mutable half of an ensemble definition — the options `create` and
/// `configure` share. `-command` is create-only and `-namespace` is
/// configure-read-only, so neither lives here.
struct EnsembleOptions {
    map: Vec<(String, Vec<Value>)>,
    subcommands: Option<Vec<String>>,
    prefixes: bool,
    parameters: Vec<String>,
    unknown: Option<Vec<Value>>,
}

impl EnsembleOptions {
    fn from_def(def: &crate::command::EnsembleDef) -> Self {
        Self {
            map: def.map.clone(),
            subcommands: def.subcommands.clone(),
            prefixes: def.prefixes,
            parameters: def.parameters.clone(),
            unknown: def.unknown.clone(),
        }
    }
}

/// Apply one `-option value` pair that `create` and `configure` share. The
/// caller has already resolved the option word against its own table, so this
/// only owns the value parsing (C's per-`case` bodies).
///
/// Relative `-map` targets are qualified against the current namespace, which
/// is what both C paths use: `CRT_MAP` against the ensemble's own namespace and
/// `CONF_MAP` against `TclGetCurrentNamespace(interp)` — and each is the current
/// namespace at the point its command runs.
fn apply_shared_option(
    opts: &mut EnsembleOptions,
    which: tcl_cmd_core::ensemble::SharedOption,
    val: &Value,
    vm: &mut Vm,
) -> Result<(), Completion<Value>> {
    use tcl_cmd_core::ensemble::SharedOption;
    match which {
        SharedOption::Map => {
            let pairs = vm.dict_pairs(val)?;
            let mut map = Vec::with_capacity(pairs.len());
            for (key, value) in pairs {
                let mut prefix = value.as_list().map_err(|e| err(e.message))?.to_vec();
                // C qualifies the target at *parse* time, so the qualified
                // name is what the ensemble stores, what dispatch calls, and
                // what `-map` reads back — a target left raw would be looked
                // up in whatever namespace happened to be current at call
                // time, and so would usually be uncallable. Only the target
                // word is qualified; the rest of the prefix is fixed leading
                // arguments. An empty prefix is left alone — the "must be
                // non-empty lists" check is a separate concern.
                if let Some(target) = prefix.first_mut() {
                    *target = Value::string(vm.qualify_name(&target.to_str()));
                }
                // Dict semantics for a repeated key: the last value wins but
                // keeps the first occurrence's position, so `-map` reads back
                // in the order the keys first appeared.
                map.push((key, prefix));
            }
            tcl_cmd_core::ensemble::validate_map_targets(&map)
                .map_err(|e| err(e.into_message()))?;
            opts.map = map;
        }
        SharedOption::Subcommands => {
            let elems = val.as_list().map_err(|e| err(e.message))?;
            opts.subcommands =
                (!elems.is_empty()).then(|| elems.iter().map(|v| v.to_str().to_string()).collect());
        }
        SharedOption::Parameters => {
            let elems = val.as_list().map_err(|e| err(e.message))?;
            opts.parameters = elems.iter().map(|v| v.to_str().to_string()).collect();
        }
        SharedOption::Prefixes => {
            opts.prefixes = val.as_bool().map_err(|e| err(e.message))?;
        }
        SharedOption::Unknown => {
            let elems = val.as_list().map_err(|e| err(e.message))?;
            opts.unknown = (!elems.is_empty()).then(|| elems.to_vec());
        }
    }
    Ok(())
}

/// `namespace ensemble create ?option value ...?` — build the ensemble command.
///
/// C checks the pair arity *before* looking at any option word (`if (objc & 1)`
/// → `wrong # args`, `tclEnsemble.c:192-196`), then resolves each option
/// through `ensembleCreateOptions` with `Tcl_GetIndexFromObj` flags `0` — a
/// table that has `-command` and, deliberately, **no** `-namespace`: an
/// ensemble is always created over the namespace the command runs in.
fn ns_ensemble_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use crate::command::EnsembleDef;
    use tcl_cmd_core::ensemble::CreateOption;
    if !args.len().is_multiple_of(2) {
        return err("wrong # args: should be \"namespace ensemble create ?option value ...?\"");
    }
    let ns = vm.current_ns().to_string();
    let mut command: Option<String> = None;
    let mut opts = EnsembleOptions {
        map: Vec::new(),
        subcommands: None,
        prefixes: true,
        parameters: Vec::new(),
        unknown: None,
    };
    for pair in args.as_chunks::<2>().0 {
        let word = pair[0].to_str();
        let resolved = match CreateOption::resolve(word.as_bytes()) {
            Ok(resolved) => resolved,
            Err(message) => return err(String::from_utf8_lossy(&message).into_owned()),
        };
        let Some(shared) = resolved.shared() else {
            // `-command` names the command rather than configuring it.
            command = Some(pair[1].to_str().to_string());
            continue;
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm) {
            return completion;
        }
    }
    // The default command is the namespace itself; an explicit -command binds in
    // the current namespace when unqualified.
    let cmd_key = match command {
        Some(c) => vm.qualify_name(&c),
        None => ns.clone(),
    };
    let def = EnsembleDef {
        namespace: ns,
        map: opts.map,
        subcommands: opts.subcommands,
        prefixes: opts.prefixes,
        parameters: opts.parameters,
        unknown: opts.unknown,
    };
    vm.register_namespace_ensemble(
        &cmd_key,
        &std::rc::Rc::new(tcl_cmd_core::ensemble::EnsembleToken::new(
            def,
            display_ns(&cmd_key),
        )),
    );
    ok(Value::string(display_ns(&cmd_key)))
}

/// `namespace ensemble configure cmdname ?-option? ?value ...?`
/// (`tclEnsemble.c:377-630`): with no options a dict of every setting, with a
/// single option that option's value, otherwise `-option value` updates.
/// `ensembleConfigOptions` differs from the create table — it carries
/// `-namespace` (readable, never writable) and has no `-command`.
fn ns_ensemble_configure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use tcl_cmd_core::ensemble::ConfigOption;
    const USAGE: &str = "wrong # args: should be \"namespace ensemble configure cmdname ?-option value ...? ?arg ...?\"";
    let Some((cmd_val, rest)) = args.split_first() else {
        return err(USAGE);
    };
    // C's arity gate: one bare option word is a read, anything else must be
    // `-option value` pairs.
    if rest.len() > 1 && !rest.len().is_multiple_of(2) {
        return err(USAGE);
    }
    let written = cmd_val.to_str().to_string();
    // `Tcl_FindEnsemble` reports the two failures separately: a name that
    // resolves to no command at all is `unknown command "x"` (the
    // `Tcl_FindCommand` miss), while a name that *is* a command but carries a
    // different implementation is `"x" is not an ensemble command`. Collapsing
    // them into the second message misreports a plain typo.
    let token = match vm.lookup_command(&written) {
        Some(crate::command::Command::Ensemble(token)) => token,
        Some(_) => {
            return lookup_error(
                format!("\"{written}\" is not an ensemble command"),
                "ENSEMBLE",
                &written,
            );
        }
        None => {
            return lookup_error(
                format!("unknown command \"{written}\""),
                "COMMAND",
                &written,
            );
        }
    };
    let def = token.config();
    if rest.is_empty() {
        let mut pairs: Vec<Value> = Vec::new();
        for option in ConfigOption::all() {
            pairs.push(Value::string(option.name()));
            pairs.push(ensemble_option_value(&def, option));
        }
        return ok(Value::list(pairs));
    }
    if let [only] = rest {
        return match ConfigOption::resolve(only.to_str().as_bytes()) {
            Ok(option) => ok(ensemble_option_value(&def, option)),
            Err(message) => err(String::from_utf8_lossy(&message).into_owned()),
        };
    }
    let mut opts = EnsembleOptions::from_def(&def);
    // `apply_shared_option` qualifies relative `-map` targets against `vm`'s
    // current namespace, which is what CONF_MAP wants here: it uses
    // `TclGetCurrentNamespace(interp)` — the namespace current at the
    // `configure` call, NOT the ensemble's own namespace (which CRT_MAP uses
    // at create time). They coincide in the common
    // `namespace eval M {namespace ensemble configure …}` shape, but
    // configuring an ensemble from outside its namespace resolves relative
    // targets against the caller.
    for pair in rest.as_chunks::<2>().0 {
        let resolved = match ConfigOption::resolve(pair[0].to_str().as_bytes()) {
            Ok(resolved) => resolved,
            Err(message) => return err(String::from_utf8_lossy(&message).into_owned()),
        };
        let Some(shared) = resolved.shared() else {
            return err_with_code("option -namespace is read-only", "TCL ENSEMBLE READ_ONLY");
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm) {
            return completion;
        }
    }
    token.configure(crate::command::EnsembleDef {
        namespace: def.namespace.clone(),
        map: opts.map,
        subcommands: opts.subcommands,
        prefixes: opts.prefixes,
        parameters: opts.parameters,
        unknown: opts.unknown,
    });
    ok(Value::empty())
}

/// One `namespace ensemble configure` option's value.
fn ensemble_option_value(
    def: &crate::command::EnsembleDef,
    option: tcl_cmd_core::ensemble::ConfigOption,
) -> Value {
    use tcl_cmd_core::ensemble::ConfigOption;
    match option {
        ConfigOption::Namespace => Value::string(display_ns(&def.namespace)),
        ConfigOption::Prefixes => Value::bool(def.prefixes),
        ConfigOption::Parameters => Value::list(
            def.parameters
                .iter()
                .map(|p| Value::string(p.clone()))
                .collect(),
        ),
        ConfigOption::Unknown => def.unknown.clone().map_or_else(Value::empty, Value::list),
        ConfigOption::Subcommands => def.subcommands.as_ref().map_or_else(Value::empty, |subs| {
            Value::list(subs.iter().map(|s| Value::string(s.clone())).collect())
        }),
        ConfigOption::Map => {
            // Insertion order, not sorted: C reads the map back out of a Tcl
            // dict, so the order the `-map` pairs were given round-trips.
            let mut flat: Vec<Value> = Vec::with_capacity(def.map.len() * 2);
            for (key, words) in &def.map {
                flat.push(Value::string(key.clone()));
                // Targets are stored as canonical command keys (the VM drops
                // the leading `::`); C's map holds — and reads back — the
                // fully-qualified name, so restore the prefix on the target
                // word. The rest of the prefix is fixed arguments, not names.
                let mut prefix = words.clone();
                if let Some(target) = prefix.first_mut() {
                    *target = Value::string(display_ns(&target.to_str()));
                }
                flat.push(Value::list(prefix));
            }
            Value::list(flat)
        }
    }
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
    let written = ns.to_str().to_string();
    let target = canon_ns(vm, &written);
    let body = inscope_script(vm, script, extra);
    let mut call_argv = vec![Value::string("namespace"), Value::string("inscope")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(vm, &written, target, &body, call_argv)
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
    let written = ns.to_str().to_string();
    let child = canon_ns(vm, &written);
    // Multiple body args are concatenated with spaces, as a script.
    let body = body_parts
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let mut call_argv = vec![Value::string("namespace"), Value::string("eval")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(vm, &written, child, &body, call_argv)
}
