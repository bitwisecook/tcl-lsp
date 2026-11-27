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

use crate::{NativeObjectLifetimeLease, Value, Vm};
use tcl_runtime_api::Code;
use tcl_syntax::value::ValueOps;

#[test]
fn generic_each_loop_uses_the_actual_host_with_a_separate_source_policy() {
    let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
    let source = tcl_dialect::DialectProfile::irules();
    let mut vm = crate::native_fixture::core(host);
    // Case3 has no elements and an invalid body: the genuine generic loop
    // must complete without compiling or entering that unvisited body.
    let original = inputs(&mut vm, 3, false);
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    let policy = vm.native_compiler_policy();
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    let reached = vm
        .try_invoke_command("foreach", &original.argv)
        .expect("actual supplied C9 generic loop");
    assert_eq!(reached.code, Code::Ok);
    assert!(vm.execution_refusal.is_none());
    assert_eq!(vm.source_profile().cache_key(), source.cache_key());
    assert_eq!(
        vm.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    assert_eq!(vm.native_compiler_policy(), policy);
}

#[test]
fn generic_each_loop_keeps_unsupported_host_recipes_closed() {
    let source = tcl_dialect::DialectProfile::irules();
    let host = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let mut unsupported = crate::native_fixture::core(host);
    let original = inputs(&mut unsupported, 3, false);
    unsupported.set_dialect_profile(source);
    assert_eq!(
        super::each_loop(&mut unsupported, &original.argv, true).code,
        Code::Error
    );
    assert!(unsupported.pending.each_loop.is_none());
    assert_eq!(
        unsupported.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
}
struct Inputs {
    argv: Vec<Value>,
    _roots: [NativeObjectLifetimeLease; 3],
    _name: Option<NativeObjectLifetimeLease>,
    _member: Option<NativeObjectLifetimeLease>,
    _external_pins: Vec<Value>,
}
fn inputs(vm: &mut Vm, case: usize, shimmer: bool) -> Inputs {
    let strings = vm
        .native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    let string = |bytes: &[u8]| Value::new_native_string_bytes(bytes);
    let list = |values| Value::native_list_constructor(values, strings);
    let mut names = vec![string(b"v"), string(b"w")];
    let name = names[0].native_lifetime_lease();
    names.truncate(if case == 2 { 2 } else { 1 });
    let mut members = vec![string(b"A"), string(b"B"), string(b"C")];
    if case == 8 {
        members[0] = list(vec![string(b"N")]);
    }
    let member = members[0].native_lifetime_lease();
    members.truncate(match case {
        2 => 3,
        5 => 2,
        3 | 4 => 0,
        _ => 1,
    });
    let variables = list(names);
    let values = list(members);
    let body = string(if matches!(case, 3 | 4) {
        b"{"
    } else {
        b"observe"
    });
    let mut argv = vec![variables, values, body];
    let mut name = Some(name);
    let mut member = Some(member);
    if matches!(case, 3 | 4) {
        member = None;
    }
    if matches!(case, 6 | 7) {
        let variables = argv.remove(0);
        let body = argv.pop().unwrap();
        argv.clear();
        let empty = list(Vec::new());
        argv = if case == 6 {
            vec![variables, string(b"{"), empty, list(Vec::new()), body]
        } else {
            name = None;
            vec![empty, string(b"{"), variables, list(Vec::new()), body]
        };
        member = None;
    }
    if case == 9 {
        argv[0] = list(Vec::new());
        name = None;
    }
    let roots = [
        argv[0].native_lifetime_lease(),
        argv[1].native_lifetime_lease(),
        argv.last().unwrap().native_lifetime_lease(),
    ];
    let mut external = Vec::new();
    if shimmer {
        for value in [&name, &member].into_iter().flatten() {
            external.push(value.value().clone());
        }
    }
    if case == 1 {
        for root in &roots[..2] {
            vm.native_string_bytes(root.value()).unwrap();
        }
    }
    Inputs {
        argv,
        _roots: roots,
        _name: name,
        _member: member,
        _external_pins: external,
    }
}
