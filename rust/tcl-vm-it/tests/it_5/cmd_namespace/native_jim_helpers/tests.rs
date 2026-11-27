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

//! Jim's namespace helper procedures, through `info`, `rename`, `proc` and `namespace` calls.

use crate::fixture::run;
use crate::prelude::*;
use tcl_native_model::native_jim_lookup::{NativeJimNamespaceHelper, NativeJimNamespaceHelperKind};

const OBSERVED: &str = include_str!(
    "../../../../../tcl-vm/tests/data/native_jim_helper_literal_head/prefix-selection-20261015.tsv"
);

fn jim() -> &'static tcl_dialect::DialectProfile {
    tcl_vm::test_support::profile("jimtcl")
}

fn helpers() -> impl Iterator<Item = NativeJimNamespaceHelper> {
    let dialect = tcl_native_model::InvocationDialect::of_static_profile(jim());
    NativeJimNamespaceHelperKind::ALL
        .into_iter()
        .map(move |kind| dialect.native_jim_namespace_helper(kind).unwrap())
}

fn present(vm: &mut Vm, name: &str) -> bool {
    run(vm, "info", &["body", name]).0 == Code::Ok
}

fn ok(vm: &mut Vm, script: &str) -> Vec<u8> {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(completion.code, Code::Ok, "{script}");
    crate::fixture::bytes(vm, &completion.result)
}

#[test]
fn namespace_helpers_require_the_selected_source_extension_and_preserve_overlap() {
    let mut vm = crate::fixture::interpreter(jim());
    for helper in helpers() {
        assert!(!present(&mut vm, helper.name()), "{}", helper.name());
    }
    let mut vm = crate::fixture::distribution(jim());
    for helper in helpers() {
        assert_eq!(
            run(&mut vm, "info", &["body", helper.name()]),
            (
                Code::Ok,
                helper.static_extension_source().body().as_bytes().to_vec()
            ),
            "{}",
            helper.name()
        );
    }
    ok(
        &mut vm,
        "rename {namespace info} {}; proc {namespace which} {args} {return USER}",
    );
    vm.set_dialect_profile(jim());
    assert!(!present(&mut vm, "namespace info"));
    assert_eq!(
        run(&mut vm, "namespace which", &[]),
        (Code::Ok, b"USER".to_vec())
    );
}

fn row(kind: &str, phase: &str) -> Vec<&'static str> {
    OBSERVED
        .lines()
        .find_map(|line| {
            let fields: Vec<_> = line.split('|').collect();
            (fields[0] == kind && fields[1] == phase).then(|| fields[2..].to_vec())
        })
        .unwrap_or_else(|| panic!("missing original helper observation {kind}/{phase}"))
}

#[test]
fn original_namespace_helper_lookup_preserves_selector_and_argument_identity() {
    assert_eq!(OBSERVED.lines().count(), 200);
    for (label, selector, count, change) in [
        ("EXACT", b"upvar".as_slice(), 1, 0),
        ("ABBREVIATED", b"upv", 1, 0),
        ("EXACT_ARITY", b"upvar", 0, 0),
        ("ABBREVIATED_ARITY", b"upv", 0, 0),
        ("EXACT_NUL", b"upvar\0TAIL", 1, 0),
        ("ABBREVIATED_NUL", b"upv\0TAIL", 1, 0),
        ("ABBREVIATED_CANONICAL_REPLACED", b"upv", 1, 1),
        ("ABBREVIATED_SHORT_REPLACED", b"upv", 1, 2),
        ("EXACT_SHORT_REPLACED", b"upvar", 1, 2),
        ("EXACT_BOTH_REPLACED", b"upvar", 1, 3),
        ("ABBREVIATED_BOTH_REPLACED", b"upv", 1, 3),
        ("EXACT_NUL_BOTH_REPLACED", b"upvar\0TAIL", 1, 3),
    ] {
        let mut vm = crate::fixture::distribution(jim());
        if change & 1 != 0 {
            ok(
                &mut vm,
                "proc {namespace upvar} {args} {return [list FULL {*}$args]}",
            );
        }
        if change & 2 != 0 {
            ok(
                &mut vm,
                "proc {namespace upv} {args} {return [list SHORT {*}$args]}",
            );
        }
        ok(
            &mut vm,
            "proc probe {selector args} {namespace $selector {*}$args}",
        );
        for reuse in ["COLD", "REUSE"] {
            let phase = format!("{label}_{reuse}");
            let mut arguments = vec![Value::new_native_string_bytes(selector)];
            if count == 1 {
                arguments.push(Value::int(84));
            }
            let completion = crate::fixture::call(&mut vm, "probe", &arguments);
            let expected = row("RESULT", &phase);
            assert_eq!(completion.code.as_int().to_string(), expected[0], "{phase}");
            assert_eq!(
                crate::fixture::bytes(&vm, &completion.result),
                crate::fixture::unhex(expected[1]),
                "{phase}"
            );
        }
    }
}

#[test]
fn explicit_source_selection_retires_only_its_original_bindings_on_engine_change() {
    let c = tcl_vm::test_support::profile("tcl8.6");
    let mut vm = crate::fixture::distribution(jim());
    assert!(present(&mut vm, "namespace origin"));
    ok(&mut vm, "proc {namespace which} {args} {return USER}");
    assert!(vm.set_native_engine_profile(c));
    assert!(!present(&mut vm, "namespace origin"));
    assert!(vm.set_native_engine_profile(jim()));
    assert!(present(&mut vm, "namespace origin"));
    assert_eq!(
        run(&mut vm, "namespace which", &[]),
        (Code::Ok, b"USER".to_vec())
    );
}

#[test]
fn original_import_entry_completes_an_empty_inventory_after_rename() {
    // jimsh 0.84 completes each invocation with an empty result at level 0.
    for (setup, invocation) in [
        ("", "{namespace import} ::missing::*"),
        ("", "namespace import ::missing::*"),
        (
            "rename {namespace import} retainedImport",
            "retainedImport ::missing::*",
        ),
    ] {
        let mut vm = crate::fixture::distribution(jim());
        ok(&mut vm, setup);
        assert_eq!(ok(&mut vm, invocation), b"", "{invocation}");
        assert_eq!(ok(&mut vm, "info level"), b"0", "{invocation}");
    }
}

#[test]
fn import_provenance_is_not_acquired_from_user_spelling_or_copied_source() {
    let mut vm = crate::fixture::distribution(jim());
    let helper = helpers()
        .find(|helper| helper.kind() == NativeJimNamespaceHelperKind::Import)
        .unwrap();
    let copy = [
        Value::string("copiedImport"),
        Value::string(helper.parameters()),
        Value::string(helper.body()),
    ];
    assert_eq!(crate::fixture::call(&mut vm, "proc", &copy).code, Code::Ok);
    assert_eq!(
        run(&mut vm, "info", &["body", "copiedImport"]),
        (Code::Ok, helper.body().as_bytes().to_vec())
    );
    assert_eq!(
        ok(
            &mut vm,
            "rename {namespace import} retainedImport; proc {namespace import} {args} {return USER}; namespace import ignored",
        ),
        b"USER"
    );
    assert_eq!(ok(&mut vm, "retainedImport ::missing::*"), b"");
}
