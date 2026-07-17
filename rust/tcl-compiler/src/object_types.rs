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

//! Object-handle & object-collection → class-name provenance for the
//! `$obj method …` / `[dict get $objs $k] method …` patterns.
//!
//! [`object_handle_classes`] recognises a `set VAR [Factory new|create …]`
//! constructor assignment (the object-handle half of issue #748) *and* any SSA
//! value the type lattice typed `OBJECT(class)` — the latter now including a
//! handle retrieved from an object collection (`set p [dict get $pins $k]`).
//! [`object_collection_classes`] maps a `List`/`Dict` variable to the class of
//! its elements, harvested from the lattice's container element-typing.
//!
//! The maps union across scopes.  For the syntactic constructor signal that is
//! merely a highlight-precision convenience; for the collection signal it is
//! also the *interprocedural bridge* that makes issue #797 resolvable — an
//! object built into an instance-variable collection in one method is dispatched
//! from it in another, which no intraprocedural lattice can connect
//! (`experiments/mro_eval/RESULTS.md` measured 99.8% ⊤ intraprocedurally on real
//! `TclOO` corpora, factory-return / cross-method dominating).  An
//! un-provenanced receiver is still left to the generic shape-based option
//! fallback rather than resolved with a wrong-or-abstain lattice.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::value_shapes::parse_command_substitution;

/// Map every variable that holds an object handle to the set of class names it
/// can hold, across the top level, procedures, and method bodies of `cu`.
///
/// Two signals are unioned:
/// - the syntactic `set VAR [Class new|create …]` constructor assignment
///   (reliable for registry-modelled factory commands); and
/// - any SSA value typed `OBJECT(class)` by the type lattice — which now
///   additionally covers a handle *retrieved from an object collection*
///   (`set p [dict get $pins $k]`, `set p [lindex $objs $i]`) via
///   `type_infer`'s container element-typing.
///
/// Keys are the handle text a `$VAR method` dispatch presents once its leading
/// `$` is stripped — a scalar name (`chart`) or an array element (`arr(key)`).
/// The map unions across scopes: a highlight-only consumer does not need
/// per-scope precision, and a variable named `chart` that is a
/// `ticklecharts::chart` in one proc is overwhelmingly one in another.
#[must_use]
pub fn object_handle_classes(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        harvest_unit(fu, registry, &mut out);
    }
    // VTA-lite object-flow propagation.  Having seeded the handles that are
    // locally provable (constructor assignments + SSA `OBJECT` values), push
    // those classes along the type-propagation edges of Variable Type Analysis
    // (Sundaresan et al., OOPSLA'00) to a bounded fixpoint:
    //   - *aliasing*         `set A $B`            → A ⊇ classes(B)
    //   - *proc return*      `set A [make …]`      → A ⊇ return-class(make)
    //   - *proc parameter*   `f $obj`              → f's param ⊇ classes($obj)
    //   - *constructor param* `C new $obj`         → C's ctor param ⊇ classes($obj)
    // Nodes are name-keyed (field-based, object-insensitive) and joins are set
    // union — the economy VTA trades precision for.  Highlight-only, matching
    // the imprecision tolerance documented above.
    propagate_object_flow(cu, registry, &mut out);
    out
}

/// VTA-lite object-flow fixpoint.  Propagates object classes from the seeded
/// handles along four kinds of type-propagation edge — assignment (aliasing),
/// proc return, proc parameter, and constructor parameter — until no new class
/// reaches any node.  Nodes are name-keyed (a variable / parameter is one node
/// regardless of scope, VTA's field-based object-insensitive economy) and the
/// join is set union.  Highlight-only and union-imprecise, matching the rest of
/// this module.
fn propagate_object_flow(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    out: &mut HashMap<String, HashSet<String>>,
) {
    // Callee proc qualified name → returned object class (the factory-proc
    // signal `set c [makeThing]`).
    let returns: HashMap<&str, &str> = cu
        .procedures
        .values()
        .filter_map(|fu| {
            (fu.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(fu.return_type.class_name())
                .flatten()
                .map(|c| (fu.name.as_str(), c))
        })
        .collect();
    // Callee proc qualified name → parameter names.
    let proc_params: HashMap<&str, &[String]> = cu
        .ir_module
        .procedures
        .values()
        .map(|p| (p.qualified_name.as_str(), p.params.as_slice()))
        .collect();
    // Class qualified name → constructor parameter names (keyed
    // `::Class::<constructor>` in the IR module).
    let ctor_params: HashMap<&str, &[String]> = cu
        .ir_module
        .methods
        .iter()
        .filter_map(|(k, m)| {
            k.ends_with("::<constructor>")
                .then_some((m.class_name.as_str(), m.params.as_slice()))
        })
        .collect();
    if returns.is_empty() && proc_params.is_empty() && ctor_params.is_empty() {
        return;
    }

    // Bounded fixpoint: a handful of rounds cover realistic alias/call chains
    // without risking a runaway on a cyclic call graph.
    for _ in 0..6 {
        let bindings = scan_flow_edges(cu, registry, out, &returns, &proc_params, &ctor_params);
        let mut changed = false;
        for (name, classes) in bindings {
            let entry = out.entry(name).or_default();
            for c in classes {
                changed |= entry.insert(c);
            }
        }
        if !changed {
            break;
        }
    }
}

/// One round of the VTA-lite fixpoint: scan every statement, reading current
/// node classes from `out`, and return the `(node, classes)` bindings the
/// type-propagation edges imply this round.  The caller unions them into `out`
/// and iterates until nothing changes.
fn scan_flow_edges(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    out: &HashMap<String, HashSet<String>>,
    returns: &HashMap<&str, &str>,
    proc_params: &HashMap<&str, &[String]>,
    ctor_params: &HashMap<&str, &[String]>,
) -> Vec<(String, HashSet<String>)> {
    // Resolve a proc call head to its callee key, tolerating the `::` global
    // qualifier the way `CommandRegistry::get` does.
    let resolve_proc = |cmd: &str, canonical: Option<&str>| -> Option<&str> {
        for cand in [canonical, Some(cmd)].into_iter().flatten() {
            if let Some((k, _)) = proc_params.get_key_value(cand) {
                return Some(*k);
            }
        }
        let q = format!("::{}", cmd.trim_start_matches("::"));
        proc_params.get_key_value(q.as_str()).map(|(k, _)| *k)
    };
    let resolve_return = |cmd: &str| -> Option<&str> {
        if let Some((k, _)) = returns.get_key_value(cmd) {
            return Some(*k);
        }
        let q = format!("::{}", cmd.trim_start_matches("::"));
        returns.get_key_value(q.as_str()).map(|(k, _)| *k)
    };
    // Resolve a constructor-call head (`Pin`, `::ns::Pin`) to a class that
    // declares a constructor.  Prefers an exact / `::`-qualified match, then
    // falls back to the trailing name segment (union-imprecise, acceptable for
    // highlighting).
    let resolve_ctor_class = |head: &str| -> Option<String> {
        if ctor_params.contains_key(head) {
            return Some(head.to_owned());
        }
        let q = format!("::{}", head.trim_start_matches("::"));
        if ctor_params.contains_key(q.as_str()) {
            return Some(q);
        }
        let tail = head.rsplit("::").next().unwrap_or(head);
        ctor_params
            .keys()
            .find(|k| k.rsplit("::").next() == Some(tail))
            .map(|k| (*k).to_owned())
    };

    let mut bindings: Vec<(String, HashSet<String>)> = Vec::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                match stmt {
                    Statement::AssignValue { name, value, .. } => {
                        let v = value.trim();
                        // Aliasing edge: `set A $B` copies B's classes to A.
                        if let Some(src) = deref_arg_var(v)
                            && let Some(classes) = out.get(src).filter(|s| !s.is_empty())
                        {
                            bindings.push((name.clone(), classes.clone()));
                        }
                        // Proc-return edge: `set A [make …]`.
                        if v.starts_with('[')
                            && let Some((cmd, args)) = parse_command_substitution(v)
                        {
                            if let Some(callee) = resolve_return(&cmd) {
                                bindings.push((
                                    name.clone(),
                                    std::iter::once(returns[callee].to_owned()).collect(),
                                ));
                            }
                            // Constructor-parameter edge for a nested
                            // `[Class new …]` / `[Class create …]`.
                            emit_ctor_param_bindings(
                                &cmd,
                                &args,
                                &resolve_ctor_class,
                                ctor_params,
                                out,
                                registry,
                                &mut bindings,
                            );
                        }
                    }
                    Statement::Call {
                        command,
                        canonical_command,
                        args,
                        ..
                    } => {
                        // Proc-parameter edge: `f $obj` binds f's params.
                        if let Some(callee) = resolve_proc(command, canonical_command.as_deref()) {
                            let params = proc_params[callee];
                            for (i, arg) in args.iter().enumerate() {
                                let Some(pname) = params.get(i) else { break };
                                if pname == "args" {
                                    break;
                                }
                                if let Some(classes) = arg_classes(arg, out, registry) {
                                    bindings.push((pname.clone(), classes));
                                }
                            }
                        }
                        // Constructor-parameter edge: `Class create NAME …`.
                        emit_ctor_param_bindings(
                            command,
                            args,
                            &resolve_ctor_class,
                            ctor_params,
                            out,
                            registry,
                            &mut bindings,
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    bindings
}

/// Bind a constructor's parameters to the object classes of its arguments for a
/// `Class new ARGS…` / `Class create NAME ARGS…` call.  `head` is the class
/// command; `verb_and_args` is everything after it (`["new", "$x"]` /
/// `["create", "foo", "$x"]`).
fn emit_ctor_param_bindings(
    head: &str,
    verb_and_args: &[String],
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    ctor_params: &HashMap<&str, &[String]>,
    out: &HashMap<String, HashSet<String>>,
    registry: &CommandRegistry,
    bindings: &mut Vec<(String, HashSet<String>)>,
) {
    let ctor_args: &[String] = match verb_and_args.split_first() {
        Some((verb, rest)) if verb == "new" => rest,
        // `create NAME …` — skip the instance name.
        Some((verb, rest)) if verb == "create" => match rest.split_first() {
            Some((_name, args)) => args,
            None => return,
        },
        _ => return,
    };
    if ctor_args.is_empty() {
        return;
    }
    let Some(class) = resolve_ctor_class(head) else {
        return;
    };
    let Some(params) = ctor_params.get(class.as_str()) else {
        return;
    };
    for (i, arg) in ctor_args.iter().enumerate() {
        let Some(pname) = params.get(i) else { break };
        if pname == "args" {
            break;
        }
        if let Some(classes) = arg_classes(arg, out, registry) {
            bindings.push((pname.clone(), classes));
        }
    }
}

/// The object classes an argument denotes: a tracked `$var` handle, or a direct
/// `[Class new]` registry constructor.  `None` when the argument is not a known
/// object.
fn arg_classes(
    arg: &str,
    out: &HashMap<String, HashSet<String>>,
    registry: &CommandRegistry,
) -> Option<HashSet<String>> {
    if let Some(classes) = deref_arg_var(arg)
        .and_then(|v| out.get(v))
        .filter(|s| !s.is_empty())
    {
        return Some(classes.clone());
    }
    constructor_class(arg, registry).map(|c| std::iter::once(c.to_string()).collect())
}

/// The variable name a `$name` / `${name}` argument dereferences, or `None` for
/// a non-plain reference.
fn deref_arg_var(text: &str) -> Option<&str> {
    let rest = text.trim().strip_prefix('$')?;
    Some(
        rest.strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(rest),
    )
}

/// Map every variable that holds an object *collection* — a `List`/`Dict`
/// whose elements are all `OBJECT(class)` — to the set of element class names,
/// read out of the SSA type lattice across the top level, procedures, and
/// method bodies of `cu`.
///
/// Keys are SSA variable names.  The map unions across scopes, which is the
/// interprocedural bridge the intraprocedural lattice cannot make on its own:
/// a `Pins` instance variable filled with `[Pin new]` handles in one method is
/// thereby known to be a `Dict` of `Pin` at a `[dict get $Pins $k] method …`
/// dispatch in a *different* method — the exact `SpiceGenTcl` shape from issue
/// #797.  Highlight-only, matching the imprecision tolerance of
/// [`object_handle_classes`].
#[must_use]
pub fn object_collection_classes(cu: &CompilationUnit) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        for ((sym, _ver), t) in &fu.types {
            if let Some(class) = t.element_class() {
                out.entry(fu.ssa.var_name(*sym).to_owned())
                    .or_default()
                    .insert(class.to_owned());
            }
        }
    }
    out
}

fn harvest_unit(
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    out: &mut HashMap<String, HashSet<String>>,
) {
    // Syntactic constructor assignments (`set VAR [Class new|create …]`) and
    // naming object-factories (`struct::graph myG` — the created object command
    // is `myG`, not a `set` target the SSA lattice can carry).
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignValue { name, value, .. } => {
                    if let Some(class) = constructor_class(value, registry) {
                        out.entry(name.clone())
                            .or_default()
                            .insert(class.to_string());
                    }
                }
                // A registry naming factory (`creates_instance_at` + a class)
                // names the new object command positionally, e.g. `struct::graph
                // myG` / `struct::tree myT`.  Only a plain bareword name binds
                // (the `= | := | as | deserialize` operator forms and dynamic
                // `$name` do not create a statically-known handle).
                Statement::Call { command, args, .. } => {
                    if let Some(spec) = registry.get(command)
                        && let Some(idx) = spec.creates_instance_at
                        && let Some(oc) = spec.object_class
                        && let Some(name) = args.get(idx as usize)
                        && is_plain_object_name(name)
                    {
                        out.entry(name.clone())
                            .or_default()
                            .insert(oc.class_name.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    // SSA values typed `OBJECT(class)` — includes collection retrievals
    // (`set p [dict get $pins $k]`) the syntactic scan above cannot see.
    for ((sym, _ver), t) in &fu.types {
        if t.tcl_type() == Some(tcl_registry::TclType::Object)
            && let Some(class) = t.class_name()
        {
            out.entry(fu.ssa.var_name(*sym).to_owned())
                .or_default()
                .insert(class.to_owned());
        }
    }
}

/// The registry class named by a `[Class new|create …]` constructor value, or
/// `None` when the value is not such a call.  A `TclOO` class command may be
/// written with or without the leading `::` global qualifier; the registry's
/// [`CommandRegistry::object_class`] strips it as [`CommandRegistry::get`] does.
fn constructor_class<'r>(value: &str, registry: &'r CommandRegistry) -> Option<&'r str> {
    let (head, args) = parse_command_substitution(value.trim())?;
    // `[Class new|create …]` — or a registry naming factory returning its own
    // instance (`[struct::graph]` / `[struct::graph name]`), matching the SSA
    // lattice's `return_type_for_command` object-factory typing so the syntactic
    // and lattice signals agree.
    if args.first().is_some_and(|s| s == "new" || s == "create") {
        return registry.object_class(&head).map(|c| c.class_name);
    }
    registry
        .get(&head)
        .filter(|s| s.creates_instance_at.is_some())
        .and_then(|s| s.object_class)
        .map(|c| c.class_name)
}

/// Whether `name` is a plain object-command name a naming factory binds — a
/// bareword, not a `$var` / `[subst]` and not one of the `struct` deserialise
/// operator words (`= | := | as | deserialize`) that occupy the name slot when
/// the object is built from a source instead of freshly named.
fn is_plain_object_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['$', '[', '{'])
        && !matches!(name, "=" | ":=" | "as" | "deserialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    #[test]
    fn bareword_widget_path_is_a_handle() {
        // `ttk::treeview .t` is syntactically identical to the tcllib
        // naming-factory shape (`struct::graph g`) `harvest_unit` already
        // reads generically via `creates_instance_at`/`object_class` — so a
        // Tk widget's bareword path becomes a tracked handle with zero new
        // code in this pass, once the registry declares those two fields
        // (issue #927).
        let registry = CommandRegistry::build_default();
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get(".t").map(|s| s.contains("ttk::treeview")),
            Some(true),
            "`.t` should be a tracked ttk::treeview handle; got {map:?}"
        );
    }

    #[test]
    fn var_captured_widget_path_is_a_handle() {
        let registry = CommandRegistry::build_default();
        let src = "set lb [listbox .l]\n$lb curselection\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("lb").map(|s| s.contains("listbox")),
            Some(true),
            "`lb` should be a tracked listbox handle; got {map:?}"
        );
    }

    #[test]
    fn scalar_handle_from_constructor() {
        let registry = CommandRegistry::build_default();
        let src = "set chart [ticklecharts::chart new]\n$chart Xaxis -name x\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("chart").map(|s| s.contains("ticklecharts::chart")),
            Some(true),
            "chart should be tracked as a ticklecharts::chart handle; got {map:?}"
        );
    }

    #[test]
    fn registry_naming_factory_handles() {
        // A registry object-factory names its instance either positionally
        // (`struct::graph g`) or as a substitution result (`set g [struct::graph]`)
        // — both must be tracked as `struct::graph` handles for `$g walk …`
        // method-callback resolution.
        let registry = CommandRegistry::build_default();
        for (src, key) in [
            ("struct::graph myG\nmyG walk root -command cb\n", "myG"),
            ("set g [struct::graph]\n$g walk root -command cb\n", "g"),
            ("struct::tree myT\nmyT walkproc root cb\n", "myT"),
        ] {
            let cu = CompilationUnit::build_for(src, &registry, false);
            let map = object_handle_classes(&cu, &registry);
            let class = if src.contains("tree") {
                "struct::tree"
            } else {
                "struct::graph"
            };
            assert_eq!(
                map.get(key).map(|s| s.contains(class)),
                Some(true),
                "`{src}` should track {key} as a {class} handle; got {map:?}"
            );
        }
    }

    #[test]
    fn registry_factory_operator_form_binds_nothing() {
        // `struct::graph = $serial` puts a deserialise *operator* (`=`) in the
        // `?name?` slot — it names no object command, so neither
        // `object_handle_classes` nor the analyser's `instance_classes` may bind
        // it (a bogus `=` handle would suppress real W123/W307 and mis-resolve a
        // command literally named `=`).
        let registry = CommandRegistry::build_default();
        for op in ["=", ":=", "as", "deserialize"] {
            let src = format!("struct::graph {op} $serial\n");
            let cu = CompilationUnit::build_for(&src, &registry, false);
            assert!(
                !object_handle_classes(&cu, &registry).contains_key(op),
                "`struct::graph {op}` must not track `{op}` as an object handle"
            );
            let r = crate::analyser::Analyser::new().analyse(&src, "tcl9.0");
            assert!(
                !r.instance_classes.contains_key(op),
                "`struct::graph {op}` must not bind `{op}` in instance_classes"
            );
        }
    }

    #[test]
    fn non_constructor_assignment_is_not_a_handle() {
        let registry = CommandRegistry::build_default();
        let src = "set x [expr {1 + 2}]\nset y hello\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert!(
            map.is_empty(),
            "no object handles expected for non-constructor assignments; got {map:?}"
        );
    }

    #[test]
    fn interproc_param_from_object_arg_is_a_handle() {
        // `set p [Pin new]; connect $p` — the object flows into `connect`'s
        // parameter `dev`, so `$dev method …` in the body resolves (the
        // param-receiver case the mro_eval experiment measured as 60% of ⊤).
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   proc connect {dev} { $dev cfg }\n\
                   set p [Pin new]\n\
                   connect $p\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("dev").map(|s| s.contains("::Pin")),
            Some(true),
            "connect's param `dev` should be a ::Pin handle; got {map:?}"
        );
    }

    #[test]
    fn interproc_param_flows_through_call_chain() {
        // `a $p` → `b $x` → `$y cfg`: the class flows two hops through the
        // fixpoint, so the innermost param `y` is a ::Pin handle.
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   proc a {x} { b $x }\n\
                   proc b {y} { $y cfg }\n\
                   set p [Pin new]\n\
                   a $p\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("y").map(|s| s.contains("::Pin")),
            Some(true),
            "the class should flow a→b so `y` is a ::Pin handle; got {map:?}"
        );
    }

    #[test]
    fn aliasing_copies_handle_class() {
        // `set a [Pin new]; set b $a` — the plain var-ref assignment aliases
        // `a`'s class onto `b`, so `$b method …` resolves (VTA assignment edge).
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   set a [Pin new]\n\
                   set b $a\n\
                   $b cfg\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("b").map(|s| s.contains("::Pin")),
            Some(true),
            "`b` should alias `a`'s ::Pin class; got {map:?}"
        );
    }

    #[test]
    fn constructor_param_typed_from_object_arg() {
        // Case B — an object passed *into* a constructor: `Wrap new $p` binds
        // the constructor's parameter `inner` to ::Pin, so `$inner method …`
        // inside the constructor body resolves (VTA constructor-param edge).
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   oo::class create Wrap {\n\
                     variable held\n\
                     constructor {inner} { $inner cfg; set held $inner }\n\
                   }\n\
                   set p [Pin new]\n\
                   set w [Wrap new $p]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("inner").map(|s| s.contains("::Pin")),
            Some(true),
            "constructor param `inner` should be a ::Pin handle; got {map:?}"
        );
        // …and the aliasing edge carries it onto the instance var it is stored
        // in, so a *different* method dispatching `$held m` resolves too.
        assert_eq!(
            map.get("held").map(|s| s.contains("::Pin")),
            Some(true),
            "instance var `held` should alias the ::Pin ctor param; got {map:?}"
        );
    }

    #[test]
    fn instance_var_from_constructor_param_bridges_methods() {
        // The full Case-B shape: an object flows in through the constructor,
        // is stored in an instance variable, and is dispatched on from an
        // unrelated method.  ctor-param + aliasing edges together resolve it.
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Motor { method spin {args} {} }\n\
                   oo::class create Car {\n\
                     variable engine\n\
                     constructor {e} { set engine $e }\n\
                     method go {} { $engine spin }\n\
                   }\n\
                   set m [Motor new]\n\
                   set c [Car new $m]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("engine").map(|s| s.contains("::Motor")),
            Some(true),
            "instance var `engine` should be a ::Motor handle via ctor param; got {map:?}"
        );
    }

    #[test]
    fn snit_named_constructor_types_handle() {
        // `set o [foo create x]` for a snit type types `o` as the snit class,
        // so `$o method` resolves.  Requires the signature scan to record snit
        // types as known classes (a pure-snit file has no "class"/"oo::").
        let registry = CommandRegistry::build_default();
        let src = "snit::type foo { method smeth {} {} }\nset o [foo create x]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("o").map(|s| s.contains("::foo")),
            Some(true),
            "`o` from `foo create x` should be a ::foo handle; got {map:?}"
        );
    }

    #[test]
    fn collection_of_objects_is_tracked() {
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin {}\n\
                   dict set pins a [Pin new]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_collection_classes(&cu);
        assert_eq!(
            map.get("pins").map(|s| s.contains("::Pin")),
            Some(true),
            "pins should be a collection of ::Pin; got {map:?}"
        );
    }

    #[test]
    fn collection_class_bridges_across_methods() {
        // The interprocedural case from issue #797: one method fills the `pins`
        // collection, a *different* method dispatches on an element.  The
        // cross-scope union makes `pins` a collection-of-Pin at both sites.
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   oo::class create Dev {\n\
                     variable pins\n\
                     method add {k} { dict set pins $k [Pin new] }\n\
                     method use {k} { [dict get $pins $k] cfg -node 1 }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_collection_classes(&cu);
        assert_eq!(
            map.get("pins").map(|s| s.contains("::Pin")),
            Some(true),
            "pins should be a collection of ::Pin, harvested from method add; got {map:?}"
        );
    }

    #[test]
    fn spicegentcl_configurable_device_shape_resolves() {
        // The exact SpiceGenTcl shape: namespaced `oo::configurable` classes,
        // the collection built and dispatched in the *same* big method's switch
        // arms with fully-qualified constructors.  Locks in that an
        // `oo::configurable` class body is lowered (so its `[::ns::Pin new]`
        // writes type the `Pins` dict) — issue #797.
        let registry = CommandRegistry::build_default();
        let src = "namespace eval ::SpiceGenTcl {\n\
                     oo::configurable create Pin { property node }\n\
                     oo::configurable create Device {\n\
                       variable Pins\n\
                       method actOnPin {action pin node} {\n\
                         switch -- $action {\n\
                           add { dict append Pins $pin [::SpiceGenTcl::Pin new $pin $node] }\n\
                           node { [dict get $Pins $pin] configure -node $node }\n\
                         }\n\
                       }\n\
                     }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_collection_classes(&cu);
        assert_eq!(
            map.get("Pins").map(|s| s.contains("::SpiceGenTcl::Pin")),
            Some(true),
            "Pins should be a collection of ::SpiceGenTcl::Pin; got {map:?}"
        );
    }
}
