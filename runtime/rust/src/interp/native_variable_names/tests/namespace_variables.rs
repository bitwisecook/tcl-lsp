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

//! Namespace declarations, alias settlement, qualified scope and the namespace cell behind a compiled local.

use super::*;

#[test]
fn generic_namespace_declarations_match_all_25_native_execution_results() {
    let source = include_bytes!(
        "../../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/source.tcl"
    );
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!(
                "../../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.5.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.6.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl9.0.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl9.1.txt"
            )
            .as_slice(),
        ),
    ] {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

// Native proof: naming.variable.namespace-alias-settlement-after-store
// docs/design/analysis/name-resolution-proofs/variable.namespace-alias-settlement-after-store.md
#[test]
fn namespace_alias_settlement_matches_all_15_native_callback_and_error_results() {
    let source =
        include_bytes!("../../../../tests/data/native_namespace_alias_settlement/source.tcl");
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_namespace_alias_settlement/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_namespace_alias_settlement/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_namespace_alias_settlement/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_namespace_alias_settlement/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_namespace_alias_settlement/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

#[test]
fn qualified_variable_scope_matches_all_six_native_sequences() {
    let source =
        include_bytes!("../../../../tests/data/native_variable_qualified_target/source.tcl");
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_variable_qualified_target/tcl8.4.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_variable_qualified_target/tcl8.5.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_variable_qualified_target/tcl8.6.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_variable_qualified_target/tcl9.0.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_variable_qualified_target/tcl9.1.txt")
                .as_slice(),
        ),
        (
            "jim",
            include_bytes!("../../../../tests/data/native_variable_qualified_target/jim.txt")
                .as_slice(),
        ),
    ] {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

fn namespace_cache_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let original = argv[1];
    assert_eq!(
        super::super::super::obj_bytes(interp.read_original_c_variable(original).unwrap()),
        b"LOCAL"
    );
    let local_slot = interp.original_c_local_index(original).unwrap();
    let before = interp
        .frames
        .borrow()
        .native_compiled_cell_identity(local_slot)
        .unwrap();
    let level = interp.current_level();
    let alias_slot = interp
        .frames
        .borrow()
        .native_compiled_name_index(level, b"alias")
        .unwrap();
    let code = interp.link_original_compiled_namespace_variable(
        original,
        crate::namespace::GLOBAL,
        alias_slot,
        true,
    );
    assert_eq!(code, Code::Ok, "{:?}", interp.result_bytes());
    assert!(obj::native_variable_name::with_local(original, |_| ()).is_none());
    assert!(
        obj::native_variable_name::with_parsed(original, |cache| cache.array.is_none()).unwrap()
    );
    assert_eq!(
        interp
            .frames
            .borrow()
            .native_compiled_cell_identity(local_slot),
        Some(before)
    );
    assert_eq!(
        super::super::super::obj_bytes(interp.var_get(b"v").unwrap()),
        b"LOCAL"
    );
    let selected = interp
        .prepare_original_c_name_in(
            original,
            NativeVariableNameLookupPurpose::Read,
            Some(crate::namespace::GLOBAL),
        )
        .unwrap()
        .unwrap();
    let (receiver, _) = interp
        .capture_original_c_selection(original, &selected, NativeVariableNameLookupPurpose::Read)
        .unwrap()
        .unwrap();
    assert_eq!(
        super::super::super::obj_bytes(receiver.read().unwrap().unwrap()),
        b"GLOBAL"
    );
    interp.set_result_bytes(b"");
    Code::Ok
}

#[test]
fn namespace_opcodes_bypass_original_local_cache_and_retain_the_namespace_cell() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        crate::counters::reset();
        {
            let mut interp = Interp::with_native_core(
                super::super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"namespace_cache_probe", namespace_cache_probe);
            let original = Owned::fresh(obj::new_string_bytes(b"v"));
            interp.var_set(b"name", original.as_ptr()).unwrap();
            let code = interp.eval_str(b"set v GLOBAL; proc p {} {set v LOCAL; namespace_cache_probe $::name; return $alias}; p");
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert_eq!(interp.result_bytes(), b"GLOBAL", "{engine}");
            assert!(
                obj::native_variable_name::with_parsed(original.as_ptr(), |cache| cache
                    .array
                    .is_none())
                .unwrap()
            );
            assert!(!interp.host_refusal_pending(), "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}
