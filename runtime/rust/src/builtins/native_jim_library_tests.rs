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

//! Core registration and the selected Jim source library have distinct owners.

use crate::interp::{Command, Interp};
use crate::namespace::GLOBAL;
use tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind;
use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;
use tcl_syntax::value::ValueOps;

fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn namespace_source_selection_is_separate_from_core_and_preserves_a_replacement() {
    let mut interp = native();
    assert!(interp.find_command_id(GLOBAL, b"namespace").is_some());
    let dialect = interp.native_invocation_dialect();
    for kind in NativeJimNamespaceHelperKind::ALL {
        let helper = dialect.native_jim_namespace_helper(kind).unwrap();
        assert!(interp
            .find_command_id(GLOBAL, helper.name().as_bytes())
            .is_none());
    }
    assert_eq!(
        interp.eval_str(b"proc {namespace which} args {return USER}"),
        crate::interp::Code::Ok,
    );
    let replacement = interp.resolve_cmd_token(b"namespace which").unwrap();
    crate::cmd_proc::install_stock_scripted_library(
        &mut interp,
        NativeScriptedLibrary::NamespaceHelpers,
    );
    for kind in NativeJimNamespaceHelperKind::ALL {
        let helper = dialect.native_jim_namespace_helper(kind).unwrap();
        let generation = interp.resolve_cmd_token(helper.name().as_bytes()).unwrap();
        assert!(matches!(
            interp
                .namespaces()
                .command_by_generation(generation)
                .map(|(_, command)| command),
            Some(Command::Proc(_)),
        ));
    }
    assert_eq!(
        interp.resolve_cmd_token(b"namespace which"),
        Some(replacement)
    );
    assert_eq!(
        interp.eval_str(b"namespace which ignored"),
        crate::interp::Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"USER");
    assert!(interp.native_execution_refusal().is_none());
}

#[test]
fn selected_namespace_formal_conversion_retains_original_source_filename_and_line() {
    let mut interp = native();
    crate::cmd_proc::install_stock_scripted_library(
        &mut interp,
        NativeScriptedLibrary::NamespaceHelpers,
    );
    let context = interp.native_jim_object_context().unwrap();
    let dialect = interp.native_invocation_dialect();
    for kind in NativeJimNamespaceHelperKind::ALL {
        let helper = dialect.native_jim_namespace_helper(kind).unwrap();
        let generation = interp.resolve_cmd_token(helper.name().as_bytes()).unwrap();
        let procedure = match interp
            .namespaces()
            .command_by_generation(generation)
            .unwrap()
            .1
        {
            Command::Proc(procedure) => procedure.clone(),
            _ => panic!("selected original procedure"),
        };
        let declaration = procedure.declaration();
        let original = declaration.jim_parameters.as_ref().unwrap();
        let parameters = ValueOps::list_elements(&mut interp, &original.as_ptr()).unwrap();
        assert_eq!(parameters.len(), declaration.params.len());
        // JimCreateProcedureCmd calls Jim_ListLength on each formal. The
        // required-name wrapper is a List; its actual member owns Source.
        // See the retained Jim 0.84 C oracle and jim.c:4393/6953-6977.
        for parameter in parameters {
            let fields = ValueOps::list_elements(&mut interp, &parameter).unwrap();
            for field in fields {
                let info = crate::native_source::original_source_info(field, &context)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    crate::interp::obj_bytes(info.filename.as_ptr()),
                    b"nshelper.tcl"
                );
                assert_eq!(info.line, helper.static_extension_source().first_line());
            }
        }
    }
    assert!(interp.native_execution_refusal().is_none());
}
