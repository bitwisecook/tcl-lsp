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

//! Jim's `namespace import` of aliases, compared with jimsh 0.84.

use crate::prelude::*;

fn native() -> Vm {
    crate::fixture::distribution(tcl_vm::test_support::profile("jim"))
}
fn eval(vm: &mut Vm, script: &str) -> Vec<u8> {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(completion.code, Code::Ok, "{script}");
    crate::fixture::bytes(vm, &completion.result)
}

#[test]
fn original_vm_jim_alias_query_keeps_explicit_root_context_after_destination_cache() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // jimsh 0.84: `dest::branch`, `` (import), `list`, `dest::branch`.
    let mut vm = native();
    assert_eq!(
        eval(
            &mut vm,
            "alias src::branch list; alias dest::src::branch dest::branch; namespace eval dest {list [info alias src::branch] [namespace import ::src::branch]}",
        ),
        b"dest::branch {}"
    );
    assert_eq!(eval(&mut vm, "info alias src::branch"), b"list");
    assert_eq!(eval(&mut vm, "info commands dest::branch"), b"dest::branch");
}

#[test]
fn original_vm_jim_import_uses_cached_prefix_without_reading_original_members() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // jimsh 0.84: `list 5`, then `5 x`.
    let mut vm = native();
    crate::fixture::call(
        &mut vm,
        "alias",
        &[
            Value::string("src::branch"),
            Value::string("list"),
            Value::int(5),
        ],
    );
    assert_eq!(
        eval(
            &mut vm,
            "namespace eval dest {namespace import ::src::branch}"
        ),
        b""
    );
    assert_eq!(eval(&mut vm, "info alias src::branch"), b"list 5");
    assert_eq!(eval(&mut vm, "dest::branch x"), b"5 x");
}

#[test]
fn original_vm_jim_import_compares_whole_prefix_before_following_alias_chains() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // jimsh 0.84 gives each row's result.
    for (setup, expected) in [
        (
            "alias src::branch dest::branch EXTRA",
            b"0 {} dest::branch".as_slice(),
        ),
        (
            "alias src::branch dest::branch",
            b"1 {import pattern \"::src::branch\" would create a loop} {}",
        ),
        (
            "alias chain dest::branch; alias src::branch chain",
            b"1 {import pattern \"::src::branch\" would create a loop} {}",
        ),
    ] {
        let mut vm = native();
        let script = format!(
            "{setup}; list [catch {{namespace eval dest {{namespace import ::src::branch}}}} r] $r [info commands dest::branch]"
        );
        assert_eq!(eval(&mut vm, &script), expected, "{setup}");
    }
}

fn refuse(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    vm.refuse_host_command("original VM import first cause".to_owned())
}

#[test]
fn original_vm_jim_import_keeps_first_host_and_restored_context_terminal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    vm.register("refuse", refuse);
    eval(&mut vm, "alias src::branch list 5");
    let refusal = vm
        .try_eval_source("namespace eval dest {catch refuse; namespace import ::src::branch}")
        .expect_err("the host refusal stays terminal");
    assert!(
        format!("{refusal:?}").contains("original VM import first cause"),
        "{refusal:?}"
    );
    assert_eq!(eval(&mut vm, "info commands dest::branch"), b"");
}
