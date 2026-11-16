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
use std::{cell::Cell, rc::Rc};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};

fn invoke_export(vm: &mut Vm, patterns: &[Value]) -> Completion<Value> {
    let mut arguments = vec![Value::string("export")];
    arguments.extend_from_slice(patterns);
    // Handler getter order ends before the public evaluator logs an Error argv.
    vm.invoke_command_value_at(
        vm.current_ns_id(),
        &Value::string("namespace"),
        &arguments,
        &[],
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    )
}

fn exports(vm: &Vm) -> Vec<Vec<u8>> {
    vm.exports_get()
        .iter()
        .map(|name| name.as_bytes().to_vec())
        .collect()
}

fn pending_list(vm: &Vm) -> Value {
    Value::native_list_constructor(
        vec![Value::new_native_string_bytes(b"unreached".as_slice())],
        vm.actual_native_string_protocol().unwrap(),
    )
}

#[test]
fn export_invalid_pattern_stops_before_later_original_getter_and_keeps_prior_commits() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(release));
        vm.add_exports(&[tcl_core_types::NameBytes::from("before")]);
        let later = pending_list(&vm);
        let result = invoke_export(
            &mut vm,
            &[
                Value::new_native_string_bytes(b"-clear\0tail".as_slice()),
                Value::new_native_string_bytes(b"kept\xff".as_slice()),
                Value::new_native_string_bytes(b"::invalid\0tail".as_slice()),
                later.clone(),
            ],
        );
        assert_eq!(result.code, Code::Error, "{release}");
        assert_eq!(
            result.result.resident_string_bytes().unwrap().as_ref(),
            b"invalid export pattern \"::invalid\": pattern can't specify a namespace"
        );
        assert!(later.resident_string_bytes().is_none(), "{release}");
        assert_eq!(exports(&vm), [b"kept\xff".to_vec()], "{release}");
        assert!(vm.execution_refusal.is_none(), "{release}");
    }
}

#[test]
fn export_clear_control_is_exact_leading_and_uses_cstring_extent() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (patterns, expected) in [
            (
                vec![b"-clear\0tail".as_slice(), b"p", b"-clear"],
                vec![b"p".as_slice(), b"-clear"],
            ),
            (
                vec![b"-cl".as_slice(), b"p"],
                vec![b"before".as_slice(), b"-cl", b"p"],
            ),
            (
                vec![b"--".as_slice(), b"p"],
                vec![b"before".as_slice(), b"--", b"p"],
            ),
            (
                vec![b"-clear\xc0\x80tail".as_slice(), b"p"],
                vec![b"before".as_slice(), b"-clear\xc0\x80tail", b"p"],
            ),
            (
                vec![b"p".as_slice(), b"-clear\0tail"],
                vec![b"before".as_slice(), b"p", b"-clear"],
            ),
        ] {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(release));
            vm.add_exports(&[tcl_core_types::NameBytes::from("before")]);
            let values: Vec<_> = patterns
                .into_iter()
                .map(Value::new_native_string_bytes)
                .collect();
            let result = invoke_export(&mut vm, &values);
            assert_eq!(result.code, Code::Ok, "{release}");
            assert_eq!(exports(&vm), expected, "{release}");
            assert!(vm.execution_refusal.is_none(), "{release}");
        }
    }
}

struct FailingFormatter(Cell<usize>);
impl NativeIntegerFormatter for FailingFormatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _: NativeIntegerKind,
        _: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.0.set(self.0.get() + 1);
        Err(NativeIntegerFormatterUnavailable::Operation)
    }
}
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<FailingFormatter>,
}
impl tcl_platform::Host for FormatterHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        Some(&*self.formatter)
    }
}

#[test]
fn export_commits_each_original_pattern_before_a_later_getter_refusal() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let formatter = Rc::new(FailingFormatter(Cell::new(0)));
    vm.set_host(Rc::new(FormatterHost {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    vm.add_exports(&[tcl_core_types::NameBytes::from("before")]);
    let failing = Value::from_native_scalar_cache(
        tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Int(
            i64::MIN,
        )),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap();
    let later = pending_list(&vm);
    let result = invoke_export(
        &mut vm,
        &[
            Value::string("-clear"),
            Value::string("committed"),
            failing.clone(),
            later.clone(),
        ],
    );
    assert_eq!(result.code, Code::Error);
    assert_eq!(formatter.0.get(), 1);
    assert_eq!(exports(&vm), [b"committed".to_vec()]);
    assert!(failing.resident_string_bytes().is_none());
    assert!(later.resident_string_bytes().is_none());
    let first = vm.execution_refusal.as_ref().unwrap();
    assert!(!matches!(
        first,
        tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
    ));
}

#[test]
fn jim_export_noop_does_not_reach_original_operand_getters() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let original = pending_list(&vm);
    let result = invoke_export(&mut vm, &[original.clone(), Value::string("-clear")]);
    assert_eq!(result.code, Code::Ok);
    assert!(original.resident_string_bytes().is_none());
    assert!(result.result.resident_string_bytes().unwrap().is_empty());
    assert!(vm.execution_refusal.is_none());
}
