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

use std::rc::Rc;
use tcl_syntax::value::ValueOps;
use tcl_vm::{Vm, host_native::NativeHost};

#[test]
fn variable_declaration_arity_matches_each_original_worker() {
    for (environment, oracle, extra_arity) in [
        (
            "tcl8.4",
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-8.4.20/stdout.txt"
            ),
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-8.4.20/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-8.5.19/stdout.txt"
            ),
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-8.5.19/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-8.6.18/stdout.txt"
            ),
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-8.6.18/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-9.0.4/stdout.txt"
            ),
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-9.0.4/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-9.1.0/stdout.txt"
            ),
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/c-9.1.0/extra-arity-stdout.txt"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/jim-5bac7c9/stdout.txt"
            ),
            include_str!(
                "../../../experiments/name-handling/results/native-variable-array-modes/jim-5bac7c9/extra-arity-stdout.txt"
            ),
        ),
    ] {
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("matching native core");
        for (prefix, rows, operands) in [
            ("NH_CONTROL variable-root ", oracle, Vec::new()),
            (
                "NH_EXTRA declaration-three ",
                extra_arity,
                vec!["::nh_extra_a", "A", "::nh_extra_b"],
            ),
            (
                "NH_EXTRA declaration-four ",
                extra_arity,
                vec!["::nh_extra_c", "C", "::nh_extra_d", "D"],
            ),
        ] {
            let row = rows
                .lines()
                .find(|row| row.starts_with(prefix))
                .expect("reached original declaration");
            let wanted = tcl_syntax::list::split_list(row).expect("retained oracle row");
            let arguments: Vec<_> = operands
                .iter()
                .map(|value| tcl_vm::Value::new_native_string_bytes(value.as_bytes()))
                .collect();
            let completion = vm
                .try_invoke_command("variable", &arguments)
                .expect("original variable declaration");
            assert_eq!(
                completion.code.as_int().to_string(),
                wanted[2],
                "{environment}: {prefix}"
            );
            assert_eq!(
                vm.native_string_bytes(&completion.result)
                    .expect("native result getter")
                    .as_ref(),
                wanted[3].as_bytes(),
                "{environment}: {prefix}"
            );
        }
    }
}
