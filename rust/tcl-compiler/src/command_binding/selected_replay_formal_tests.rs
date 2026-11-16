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

fn invocation(parameters: &str) -> ResolvedBindingInvocation {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let arguments = vec!["target".to_owned(), parameters.to_owned(), String::new()];
    let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let facts = registry
        .commands()
        .resolve_invocation("proc", &refs, registry.commands().own_surface_query())
        .unwrap()
        .facts();
    ResolvedBindingInvocation {
        command: "proc".into(),
        source_span: tcl_lexer::Span::new(0, 1),
        facts: Box::new(facts),
        literal_arguments: arguments.iter().cloned().map(Some).collect(),
        source_scalar_arguments: vec![None; arguments.len()],
        exact_argument_count: Some(arguments.len()),
        arguments,
    }
}

#[test]
fn original_procedure_replay_validity_requires_the_selected_formal_grammar() {
    // naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    // Registry replay syntax only, without installed procedure or body entry.
    let namespace = crate::ir::ExecutionNamespace::exact("::");
    let c = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    let jim = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    for parameters in ["a::b", "a(1)"] {
        let invocation = invocation(parameters);
        assert!(matches!(
            recover_procedure_definition(&invocation, &namespace, Some(c)),
            ProcedureDefinitionReplay::KnownError
        ));
        assert!(matches!(
            recover_procedure_definition(&invocation, &namespace, Some(jim)),
            ProcedureDefinitionReplay::Recovered { .. }
        ));
        assert!(matches!(
            recover_procedure_definition(&invocation, &namespace, None),
            ProcedureDefinitionReplay::Unavailable
        ));
    }
    let duplicate = invocation("args args");
    assert!(matches!(
        recover_procedure_definition(&duplicate, &namespace, Some(jim)),
        ProcedureDefinitionReplay::KnownError
    ));
    assert!(matches!(
        recover_procedure_definition(&duplicate, &namespace, Some(c)),
        ProcedureDefinitionReplay::Recovered { .. }
    ));
}
