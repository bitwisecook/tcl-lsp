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

use crate::ir::Statement;

fn body(source: &str) -> crate::ir::Module {
    crate::lowering::lower_to_ir(
        source,
        &tcl_registry::CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        ),
    )
}

#[test]
fn catch_and_try_receivers_keep_literal_sigils_elements_and_empty_data() {
    for (name, expected) in [
        ("{$literal}", "$literal"),
        ("{${braced}}", "${braced}"),
        ("{arr(key)}", "arr(key)"),
        ("{}", ""),
    ] {
        let module = body(&format!("proc p {{}} {{catch {{}} {name}}}"));
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        let Statement::Catch { result_var, .. } = &procedure.body.statements[0] else {
            panic!("authentic source catch")
        };
        assert_eq!(result_var.as_deref(), Some(expected));
    }
    let module = body("proc p {} {try {error BOOM} on error {{$literal} {opts(key)}} {}} ");
    let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
    let Statement::Try { handlers, .. } = &procedure.body.statements[0] else {
        panic!("authentic source try")
    };
    assert_eq!(handlers[0].var_name.as_deref(), Some("$literal"));
    assert_eq!(handlers[0].options_var.as_deref(), Some("opts(key)"));
}

#[test]
fn computed_control_receiver_names_keep_runtime_dispatch() {
    for source in [
        "proc p {} {catch {} $receiver}",
        "proc p {} {try {error BOOM} on error $receivers {}}",
    ] {
        let module = body(source);
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        assert!(matches!(
            &procedure.body.statements[0],
            Statement::Call { .. } | Statement::Barrier { .. }
        ));
        assert!(
            procedure
                .body
                .retained_source_tokens_for_statement(&procedure.body.statements[0])
                .is_some()
        );
    }
}
