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

//! Scope-alias commands and what an aliased store still proves about a variable's type.

use super::*;

#[test]
fn scope_alias_commands_detected() {
    let reg = registry();
    let one = |s: &str| vec![s.to_owned()];
    assert!(is_scope_alias_call(&reg, "global", &one("g")));
    assert!(is_scope_alias_call(&reg, "variable", &one("v")));
    assert!(is_scope_alias_call(
        &reg,
        "upvar",
        &["0".into(), "x".into(), "y".into()]
    ));
    assert!(is_scope_alias_call(
        &reg,
        "namespace",
        &["upvar".into(), "ns".into(), "x".into(), "y".into()]
    ));
    assert!(is_scope_alias_call(
        &reg,
        "my",
        &["variable".into(), "count".into()]
    ));
    // A plain command (and `namespace eval`) is not a scope alias.
    assert!(!is_scope_alias_call(&reg, "set", &["x".into(), "1".into()]));
    assert!(!is_scope_alias_call(
        &reg,
        "namespace",
        &["eval".into(), "ns".into(), "body".into()]
    ));
    // `my`'s other subcommands (an arbitrary method name) are not scope
    // aliases — only the reserved `variable` word is.
    assert!(!is_scope_alias_call(&reg, "my", &["touch".into()]));
}

#[test]
fn scope_alias_declaration_preserves_unknown_contents() {
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc ::f {} { variable counter; set observed $counter; return $observed }",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    assert!(
        fu.types
            .iter()
            .all(|((name, _), _)| fu.ssa.var_name(*name) != "::counter"),
        "a declaration alone cannot write namespace contents"
    );
    assert_eq!(
        fu.return_type.kind(),
        TypeKind::Overdefined,
        "an external cell's unknown contents are not the declaration's return value; actual return {}",
        fu.return_type
    );
}

#[test]
fn actual_traced_alias_store_preserves_only_a_closed_unchanged_value() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (callback, preserves_dictionary) in [
        ("", true),
        // A callback can coerce the same value without changing its bytes
        // or dictionary contents. Current representation is a separate
        // read receipt consumed by representation diagnostics.
        ("upvar 1 dictionary destination; llength $destination", true),
        (
            "upvar 1 dictionary destination; set destination altered",
            false,
        ),
    ] {
        let source = format!(
            "proc observe args {{{callback}}}; proc f {{}} {{set dictionary [dict create k 1]; upvar 0 dictionary view; trace add variable view write observe; set dictionary [dict create k 2]; llength $view}}"
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
        let function = unit.function("::f").unwrap();
        let mut writes = Vec::new();
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if matches!(&statement.statement, Statement::AssignValue { name, .. } if name == "dictionary")
                {
                    for (&symbol, &version) in &statement.defs {
                        writes.push((
                            statement.statement.span().start(),
                            function.types.get(&(symbol, version)),
                            crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index)
                                .normal_store_contents_preserved(symbol, registry),
                        ));
                    }
                }
            }
        }
        writes.sort_by_key(|(offset, _, _)| *offset);
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[1].2, preserves_dictionary, "callback {callback:?}");
        assert_eq!(
            writes[1].1.unwrap().tcl_type() == Some(TclType::Dict),
            preserves_dictionary,
            "callback {callback:?}: {:?}",
            writes[1].1,
        );
    }
}

#[test]
fn expression_types_follow_actual_alias_spellings_at_each_read() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} {set original [expr {3.5}]; upvar 0 original view; set result [expr {$original}]; return [expr {$view}]}",
        registry,
        false,
    );
    let function = unit.function("::f").unwrap();
    assert_eq!(function.return_type.tcl_type(), Some(TclType::Double));
    let writes = function
        .ssa
        .blocks
        .values()
        .flat_map(|body| &body.statements);
    let result = writes
            .filter(|statement| matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result"))
            .flat_map(|statement| &statement.defs)
            .filter_map(|(&symbol, &version)| function.types.get(&(symbol, version)))
            .collect::<Vec<_>>();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].tcl_type(), Some(TclType::Double));
}
