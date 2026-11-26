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

//! `uplevel` from a procedure, against the pinned releases' level, body and
//! frame-restoration results.

use tcl_runtime_api::{Code, Completion};
use tcl_syntax::value::ValueOps;
use tcl_vm::{Value, Vm};

fn script(case: usize) -> Value {
    match case {
        10 => Value::new_native_string_bytes(b"error BODY".as_slice()),
        11 => Value::new_native_string_bytes(b"set prefix BEFORE; set x {".as_slice()),
        12 => Value::list(vec![
            Value::string("set"),
            Value::string("marker"),
            Value::string("VALUE"),
        ]),
        13 => Value::new_native_string_bytes(b"break".as_slice()),
        _ => Value::new_native_string_bytes(b"list [info level] [info exists local]".as_slice()),
    }
}

// Native proof: naming.variable.explicit-uplevel-level-body-cache-and-frame-restoration
// docs/design/analysis/name-resolution-proofs/variable.explicit-uplevel-level-body-cache-and-frame-restoration.md
#[test]
fn uplevel_matches_the_native_level_body_and_restore_rows() {
    let levels: [&[u8]; 14] = [
        b"#0", b"0", b"1", b"bad", b"+0", b"99", b"-1", b"1.0", b"1\0X", b"#0\0X", b"#0", b"#0",
        b"#0", b"#0",
    ];
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../../../tcl-vm/tests/data/native_explicit_uplevel/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tcl-vm/tests/data/native_explicit_uplevel/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tcl-vm/tests/data/native_explicit_uplevel/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-vm/tests/data/native_explicit_uplevel/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-vm/tests/data/native_explicit_uplevel/9.1.0.tsv"),
        ),
    ] {
        for row in rows.lines() {
            let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            let case = fields[0].parse::<usize>().unwrap();
            // Rows 3-9 hand the captured C probe a word that is not a level;
            // `uplevel`'s level is optional, so there it starts the script.
            if (3..=9).contains(&case) {
                continue;
            }
            let mut vm =
                crate::fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
            let definition = vm.invoke_command(
                "proc",
                &[
                    Value::string("p"),
                    Value::string("level script"),
                    Value::string("set local LOCAL; uplevel $level $script"),
                ],
            );
            assert_eq!(definition.code, Code::Ok, "{engine}/{case}");
            let completed = crate::fixture::call(
                &mut vm,
                "p",
                &[Value::new_native_string_bytes(levels[case]), script(case)],
            );
            assert_result(&mut vm, &completed, &fields, engine, case);
            compared += 1;
        }
    }
    assert_eq!(compared, 35);
}

fn assert_result(
    vm: &mut Vm,
    completed: &Completion<Value>,
    fields: &[&str],
    engine: &str,
    case: usize,
) {
    assert_eq!(
        completed.code.as_int().to_string(),
        fields[3],
        "{engine}/{case}: {}",
        completed.result.to_str()
    );
    assert_eq!(
        vm.native_string_bytes(&completed.result).unwrap().as_ref(),
        crate::fixture::unhex(fields[4]).as_slice(),
        "{engine}/{case}"
    );
    let level = vm.try_eval_source("info level").unwrap();
    assert_eq!(
        level.result.to_str().as_ref(),
        "0",
        "caller restored: {engine}/{case}"
    );
    if case == 12 {
        assert_eq!(
            vm.get_var_bytes(b"marker").unwrap().to_str().as_ref(),
            "VALUE"
        );
    }
}
