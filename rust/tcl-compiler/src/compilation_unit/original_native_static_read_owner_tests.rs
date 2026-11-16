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

use super::*;

const SOURCE: &str = "proc f {} {{n 0}} {return $n}; f";

fn input() -> crate::analyser::ResolvedAnalysisInput {
    tcl_spectcl::core_surfaces::ensure();
    let mut analyser = crate::analyser::Analyser::new();
    analyser.prepare_analysis_input("jim").unwrap()
}

fn unit(input: &crate::analyser::ResolvedAnalysisInput) -> CompilationUnit {
    CompilationUnit::build_with_analysis_input(
        SOURCE,
        UnitBuildOptions {
            registry: input.borrowed_context_registry().commands(),
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    )
}

fn function(unit: &CompilationUnit) -> &FunctionUnit {
    let id = unit
        .ir_module
        .original_native_declaration_body_units
        .keys()
        .next()
        .expect("genuine readonly Native body");
    unit.body_units.get(id).unwrap()
}

fn static_key(function: &FunctionUnit) -> &crate::var_resolve::VariableCellKey {
    function
            .ssa
            .cell_keys()
            .iter()
            .find(|key| {
                matches!(key, crate::var_resolve::VariableCellKey::RetainedSlot(slot)
                    if matches!(slot.as_ref(), crate::raw_binding::RawBindingSlotId::Callable {name, ..}
                        if name.as_bytes() == b"n"))
            })
            .expect("actual callable-static SSA key")
}

#[test]
fn native_static_read_receipt_keeps_the_current_unit_allocation_owner() {
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let current = unit(&input);
    let current_function = function(&current);
    let receipt = current
        .original_native_declaration_static_read_alternatives(current_function, registry)
        .expect("current source-owned static recipe");
    let key = static_key(current_function);
    assert!(receipt.owns_declared_callable_static(key, "n"));
    assert!(current.ir_module.source_entry.native_entry.is_none());
    let foreign = unit(&input);
    let foreign_key = static_key(function(&foreign));
    assert_eq!(key, foreign_key);
    assert!(!receipt.owns_declared_callable_static(foreign_key, "n"));
    let copied_function = (*current_function).clone();
    assert!(
        current
            .original_native_declaration_static_read_alternatives(&copied_function, registry)
            .is_none()
    );
    let id = current_function.body_id.unwrap();
    for changed in 0..3 {
        let mut changed_unit = current.clone();
        match changed {
            0 => changed_unit
                .ir_module
                .original_native_declaration_body_units
                .clear(),
            1 => changed_unit.ir_module.retained_source_bindings = None,
            _ => changed_unit.ir_module.source_entry.unknown_entry = true,
        }
        assert!(
            changed_unit
                .original_native_declaration_static_read_alternatives(
                    changed_unit.body_units.get(&id).unwrap(),
                    registry,
                )
                .is_none()
        );
    }
}

#[test]
fn native_static_read_receipt_refuses_wrapped_and_relabelled_cells() {
    use crate::raw_binding::RawBindingSlotId;
    use crate::var_resolve::VariableCellKey;
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let current = unit(&input);
    let function = function(&current);
    let receipt = current
        .original_native_declaration_static_read_alternatives(function, registry)
        .unwrap();
    let key = static_key(function);
    let VariableCellKey::RetainedSlot(slot) = key else {
        unreachable!()
    };
    let RawBindingSlotId::Callable { allocation, name } = slot.as_ref() else {
        unreachable!()
    };
    let callable = |allocation, name| {
        VariableCellKey::RetainedSlot(Box::new(RawBindingSlotId::Callable { allocation, name }))
    };
    let mut second = allocation.clone();
    second.incarnation = crate::command_binding::AllocationIncarnation::Second;
    let mut repeated = allocation.clone();
    repeated.incarnation = crate::command_binding::AllocationIncarnation::RepeatedFresh;
    let mut other_site = allocation.clone();
    other_site.site.offset += 1;
    let mut other_generation = allocation.clone();
    other_generation.implementation_generation += 1;
    for changed in [
        VariableCellKey::Authored("n".into()),
        VariableCellKey::Element {
            cell: Box::new(key.clone()),
            index: "element".into(),
        },
        VariableCellKey::Lifetime {
            source: 1,
            cell: Box::new(key.clone()),
        },
        callable(second, name.clone()),
        callable(repeated, name.clone()),
        callable(other_site, name.clone()),
        callable(other_generation, name.clone()),
        callable(allocation.clone(), "other".into()),
    ] {
        assert!(!receipt.owns_declared_callable_static(&changed, "n"));
    }
    assert!(!receipt.owns_declared_callable_static(key, "other"));
}

#[test]
fn direct_original_static_reads_keep_missing_and_other_body_diagnostics() {
    tcl_spectcl::core_surfaces::ensure();
    for (source, expected) in [
        (SOURCE, vec![]),
        ("set n 1; proc f {} {n} {return $n}; f", vec![]),
        ("proc f {n} {{n 0}} {return $n}; f ARG", vec![]),
        ("proc f {} {{n 0}} {return $missing}; f", vec!["$missing"]),
        (
            "proc f {} {{n 0}} {return $n}; proc g {} {return $n}; g",
            vec!["$n"],
        ),
    ] {
        for per_item in [false, true] {
            let mut analyser = crate::analyser::Analyser::new();
            let result = if per_item {
                analyser.analyse_per_item(source, "jim")
            } else {
                analyser.analyse(source, "jim")
            };
            let actual = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
                .map(|diagnostic| &source[diagnostic.span.as_range()])
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "per_item={per_item}: {source}");
        }
    }
}
