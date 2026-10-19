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

fn fixture() -> Vm {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut vm = Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("explicit original C9 core bootstrap");
    vm.set_compiler(Box::new(
        tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm.register_native_command(
        "originalPure",
        Rc::new(PureFunction {
            command: "b64encode",
            mock: "cmd_b64encode".into(),
        }),
    );
    vm
}

#[test]
fn original_pure_callback_refuses_first_utf8_word_before_later_string_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = fixture();
    let values = [
        Value::new_native_string_bytes(b"\xff".as_slice()),
        Value::int(i64::MIN),
    ];
    let counts = values
        .iter()
        .map(Value::native_object_reference_count)
        .collect::<Vec<_>>();
    assert!(values[1].resident_string_bytes().is_none());
    let error = vm
        .try_invoke_command("originalPure", &values)
        .expect_err("strict callback input refuses before Core dispatch");
    assert!(matches!(
        error,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(
                tcl_syntax::raw_string::UnicodeAccessError {
                    valid_up_to: 0,
                    error_len: Some(1)
                }
            )
        )
    ));
    assert!(values[1].resident_string_bytes().is_none());
    assert_eq!(
        values
            .iter()
            .map(Value::native_object_reference_count)
            .collect::<Vec<_>>(),
        counts
    );
}

#[test]
fn original_pure_callback_preserves_strict_nul_words_and_actual_argument_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = fixture();
    let values = [Value::string("A\0B")];
    let counts = values
        .iter()
        .map(Value::native_object_reference_count)
        .collect::<Vec<_>>();
    let result = vm.try_invoke_command("originalPure", &values).unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"QQBC"
    );
    assert_eq!(
        values
            .iter()
            .map(Value::native_object_reference_count)
            .collect::<Vec<_>>(),
        counts
    );
    assert_eq!(values[0].resident_string_bytes().unwrap().as_ref(), b"A\0B");
}
