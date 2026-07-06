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

//! Object-handle → class-name provenance for the `$obj method …` pattern.
//!
//! Recognises `set VAR [Factory new|create …]` where `Factory` is a
//! registry-modelled `TclOO` class (carries an
//! [`ObjectClassSpec`](tcl_registry::ObjectClassSpec)), so a later
//! `$VAR method …` dispatch can resolve the method's options through the
//! registry — the object-handle half of issue #748.
//!
//! This is **provenance** tracking, not class *inference*.  The class comes
//! directly from the constructor call, which is the accurate signal for the
//! trackable case: the object→class binding *lattice* was measured to add no
//! resolving power on real `TclOO` corpora (`experiments/mro_eval/RESULTS.md`
//! — 99.8% ⊤, factory-return dominating), because a receiver that arrives as a
//! proc parameter is inherently un-provenanced intraprocedurally.  Those
//! receivers are left to the generic shape-based option fallback rather than
//! resolved with a wrong-or-abstain lattice.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::value_shapes::parse_command_substitution;

/// Map every variable that holds a registry-class object handle to the set of
/// class names it can hold, harvested from `set VAR [Class new|create …]`
/// across the top level, procedures, and method bodies of `cu`.
///
/// Keys are the assignment target verbatim — a scalar name (`chart`) or an
/// array element (`arr(key)`) — matching the handle text a `$VAR method`
/// dispatch presents once its leading `$` is stripped.  The map unions across
/// scopes: a highlight-only consumer does not need per-scope precision, and a
/// variable named `chart` that is a `ticklecharts::chart` in one proc is
/// overwhelmingly one in another.
#[must_use]
pub fn registry_object_handle_classes(
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

fn harvest_unit(
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    out: &mut HashMap<String, HashSet<String>>,
) {
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
        let map = registry_object_handle_classes(&cu, &registry);
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
        let map = registry_object_handle_classes(&cu, &registry);
        assert!(
            map.is_empty(),
            "no object handles expected for non-constructor assignments; got {map:?}"
        );
    }
}
