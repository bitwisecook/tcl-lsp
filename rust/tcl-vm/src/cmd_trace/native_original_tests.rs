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

//! Original byte operands against five direct native trace measurements.

use super::*;
use std::{cell::RefCell, io::Write, rc::Rc};
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn physical_vm(version: TclVersion, output: Capture) -> Vm {
    let profile = tcl_registry::model::resolve_environment(version.dialect_name()).unit_profile();
    let mut vm = Vm::with_native_core(
        Box::new(output),
        Rc::new(crate::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("matching independently selected physical core");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    vm
}

fn check_original_source(source: &[u8], expected: impl Fn(TclVersion) -> &'static [u8]) {
    for version in TclVersion::ALL {
        let output = Capture::default();
        let mut vm = physical_vm(version, output.clone());
        let completion = vm
            .try_eval_source_bytes(source)
            .expect("original byte source admitted");
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "{version:?}: {:?}",
            completion.result
        );
        assert_eq!(
            output.0.borrow().as_slice(),
            expected(version),
            "{version:?}"
        );
    }
}

#[test]
fn opaque_command_trace_operands_match_all_five_direct_native_engines() {
    check_original_source(
        include_bytes!("../../../tcl-syntax/tests/data/native_command_trace/opaque.tcl"),
        |_| b"6362ff\n6362ff\n0\n1\n0\nenter leave rename delete\n",
    );
}

#[test]
fn counted_nul_command_trace_prefixes_preserve_native_storage_and_eval_extents() {
    check_original_source(
        include_bytes!("../../../tcl-syntax/tests/data/native_command_trace/counted_nul.tcl"),
        |version| {
            if version <= TclVersion::V8_5 {
                b"6362ff\n0\n6362ff\n0\n{} {}\n"
            } else {
                b"6362ff\n0\n6362ff\n0\nenter leave\n"
            }
        },
    );
}

#[test]
fn opaque_missing_trace_command_reports_original_bytes_without_panicking() {
    for version in TclVersion::ALL {
        let mut vm = physical_vm(version, Capture::default());
        let completion = cmd_trace(
            &mut vm,
            &[
                Value::string("info"),
                Value::string("command"),
                Value::from_native_string_bytes(b"absent\xff".as_slice()),
            ],
        );
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"unknown command \"absent\xff\""
        );
    }
}

#[test]
fn unavailable_trace_prefix_refuses_before_materializing_original_name() {
    use tcl_core_types::ROOT_NS;
    use tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer;

    let mut vm = physical_vm(TclVersion::V9_0, Capture::default());
    let prefix = vm
        .native_namespace_result_object(ROOT_NS, NativeNamespaceObjectProducer::Current)
        .unwrap();
    prefix.invalidate_native_string_for_test();
    let name = Value::int(7);
    assert!(name.resident_string_bytes().is_none());
    let completion = cmd_trace(
        &mut vm,
        &[
            Value::string("add"),
            Value::string("command"),
            name.clone(),
            Value::string("delete"),
            prefix,
        ],
    );
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert!(vm.refused_completion().is_some());
    assert!(name.resident_string_bytes().is_none());
}
