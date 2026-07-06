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
    // Interprocedural: a proc called with an object argument holds that class in
    // the corresponding parameter, so `$param method …` in the body resolves.
    harvest_interproc_param_handles(cu, registry, &mut out);
    out
}

/// Interprocedural parameter provenance (the object-handle half of the #797
/// follow-up "C" step): when a proc is called with an object argument — a `$var`
/// already tracked as a handle, or a direct `[Class new]` — its corresponding
/// parameter holds that class, so a `$param method …` dispatch in the body
/// resolves like any other handle.  Iterated to a small fixpoint so a class
/// flows through a chain of calls (`f $obj` → `g $param` → …).  Highlight-only
/// and union-imprecise, matching the rest of this module.
fn harvest_interproc_param_handles(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    out: &mut HashMap<String, HashSet<String>>,
) {
    // Callee qualified name → parameter names.
    let proc_params: HashMap<&str, &[String]> = cu
        .ir_module
        .procedures
        .values()
        .map(|p| (p.qualified_name.as_str(), p.params.as_slice()))
        .collect();
    if proc_params.is_empty() {
        return;
    }
    let resolve = |cmd: &str, canonical: Option<&str>| -> Option<&str> {
        for cand in [canonical, Some(cmd)].into_iter().flatten() {
            if let Some((k, _)) = proc_params.get_key_value(cand) {
                return Some(*k);
            }
        }
        let q = format!("::{}", cmd.trim_start_matches("::"));
        proc_params.get_key_value(q.as_str()).map(|(k, _)| *k)
    };
    // Bounded fixpoint: three rounds cover call chains without risking a runaway
    // on a cyclic call graph.
    for _ in 0..3 {
        // (param name, classes) bindings discovered this round.
        let mut bindings: Vec<(String, HashSet<String>)> = Vec::new();
        let units = std::iter::once(&cu.top_level)
            .chain(cu.procedures.values())
            .chain(cu.methods.values());
        for fu in units {
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let Statement::Call {
                        command,
                        canonical_command,
                        args,
                        ..
                    } = stmt
                    else {
                        continue;
                    };
                    let Some(callee) = resolve(command, canonical_command.as_deref()) else {
                        continue;
                    };
                    let params = proc_params[callee];
                    for (i, arg) in args.iter().enumerate() {
                        let Some(pname) = params.get(i) else {
                            break;
                        };
                        if pname == "args" {
                            break;
                        }
                        // The argument's object class: a tracked `$var` handle,
                        // or a direct `[Class new]` registry constructor.
                        let classes: Option<HashSet<String>> = deref_arg_var(arg)
                            .and_then(|v| out.get(v))
                            .filter(|s| !s.is_empty())
                            .cloned()
                            .or_else(|| {
                                constructor_class(arg, registry)
                                    .map(|c| std::iter::once(c.to_string()).collect())
                            });
                        if let Some(classes) = classes {
                            bindings.push((pname.clone(), classes));
                        }
                    }
                }
            }
        }
        let mut changed = false;
        for (pname, classes) in bindings {
            let entry = out.entry(pname).or_default();
            for c in classes {
                changed |= entry.insert(c);
            }
        }
        if !changed {
            break;
        }
    }
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
    // Syntactic constructor assignments: `set VAR [Class new|create …]`.
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            let Statement::AssignValue { name, value, .. } = stmt else {
                continue;
            };
            if let Some(class) = constructor_class(value, registry) {
                out.entry(name.clone())
                    .or_default()
                    .insert(class.to_string());
            }
        }
    }
    // SSA values typed `OBJECT(class)` — includes collection retrievals
    // (`set p [dict get $pins $k]`) the syntactic scan above cannot see.
    for ((sym, _ver), t) in &fu.types {
        if t.tcl_type == Some(tcl_registry::TclType::Object)
            && let Some(class) = &t.class_name
        {
            out.entry(fu.ssa.var_name(*sym).to_owned())
                .or_default()
                .insert(class.clone());
        }
    }
}

/// The registry class named by a `[Class new|create …]` constructor value, or
/// `None` when the value is not such a call.  A `TclOO` class command may be
/// written with or without the leading `::` global qualifier; the registry's
/// [`CommandRegistry::object_class`] strips it as [`CommandRegistry::get`] does.
fn constructor_class<'r>(value: &str, registry: &'r CommandRegistry) -> Option<&'r str> {
    let (head, args) = parse_command_substitution(value.trim())?;
    if !args.first().is_some_and(|s| s == "new" || s == "create") {
        return None;
    }
    registry.object_class(&head).map(|c| c.class_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

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
