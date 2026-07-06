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
    out
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
