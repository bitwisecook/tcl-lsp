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
use tcl_core_types::{NativeHashBytePromotion, NativeHashRecipe, NativeHashWordWidth};
use tcl_dialect::TclVersion;

fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn array_entry_ledgers_match_all_320_native_table_controls() {
    // Native proof naming.variable-table.original-array-growth-and-reinsertion:
    // docs/design/analysis/name-resolution-proofs/variable-table-original-array-growth-and-reinsertion.md
    // Native proof naming.variable-table.original-opaque-key-extents:
    // docs/design/analysis/name-resolution-proofs/variable-table-original-opaque-key-extents.md
    crate::counters::reset();
    {
        let value = obj::Owned::fresh(obj::new_wide_int_obj(7));
        let fixture = include_str!(
            "../../../../rust/tcl-core-types/tests/data/native_variable_tables/array-order.tsv"
        );
        let mut controls = 0;
        for line in fixture.lines().filter(|line| !line.starts_with('#')) {
            let row: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
            let version = match row[0] {
                "8.4.20" => TclVersion::V8_4,
                "8.5.19" => TclVersion::V8_5,
                "8.6.18" => TclVersion::V8_6,
                "9.0.4" => TclVersion::V9_0,
                "9.1.0" => TclVersion::V9_1,
                _ => panic!("unknown native fixture"),
            };
            let abi = tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(None).unwrap();
            let protocol = tcl_registry::InvocationDialect::for_version(version)
                .native_variable_table_protocol(abi)
                .unwrap();
            let mut table = VarTable::default();
            table.set_hash_recipe(Some(protocol.recipe()));
            for index in 0..row[1].parse::<usize>().unwrap() {
                let original = if row[2] == "1" && index < 3 {
                    [b"k\0x".as_slice(), b"k\xff", b"k\xc0\x80"][index].to_vec()
                } else {
                    format!("k{index:02}").into_bytes()
                };
                let input = protocol
                    .names()
                    .separate_variable_input(b"a", Some(&original));
                table
                    .store_elem(b"a", input.element().unwrap().selected(), value.as_ptr())
                    .unwrap();
            }
            if row[3] == "1" {
                table.remove_elem(b"a", b"k03");
                table.store_elem(b"a", b"k03", value.as_ptr()).unwrap();
            }
            let observed = bytes(row[4]);
            let expected: Vec<_> = observed
                .split(|&byte| byte == b' ')
                .map(<[u8]>::to_vec)
                .collect();
            assert_eq!(table.array_names(b"a").unwrap(), expected, "{line}");
            controls += 1;
        }
        assert_eq!(controls, 320);
    }
    assert_eq!(crate::counters::finalize(), 0);
}

#[test]
fn trace_shells_grow_table_without_becoming_defined_inventory() {
    // Native proof naming.variable-table.undefined-array-shell-growth:
    // docs/design/analysis/name-resolution-proofs/variable-table-undefined-array-shell-growth.md
    crate::counters::reset();
    {
        let value = obj::Owned::fresh(obj::new_wide_int_obj(7));
        let mut table = VarTable::default();
        table.set_hash_recipe(Some(NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Unsigned,
            width: NativeHashWordWidth::Bits32,
        }));
        for index in 0..3 {
            table
                .store_elem(b"a", format!("k{index:02}").as_bytes(), value.as_ptr())
                .unwrap();
        }
        for index in 3..12 {
            table
                .ensure_element_shell(b"a", format!("k{index:02}").as_bytes())
                .unwrap();
        }
        assert_eq!(
            table
                .contents(b"a")
                .unwrap()
                .borrow()
                .member_order
                .bucket_count(),
            Some(16)
        );
        assert_eq!(
            table.array_names(b"a").unwrap(),
            [b"k00".to_vec(), b"k01".to_vec(), b"k02".to_vec()]
        );
        let id = table.binding_id(b"a");
        table.cleanup_trace_shell(b"a", Some(b"k04"), id);
        assert_eq!(
            table
                .contents(b"a")
                .unwrap()
                .borrow()
                .member_order
                .bucket_count(),
            Some(16)
        );
        assert_eq!(
            table
                .contents(b"a")
                .unwrap()
                .borrow()
                .member_order
                .physical_keys()
                .len(),
            11
        );
        table.store_elem(b"a", b"k03", value.as_ptr()).unwrap();
        assert_eq!(
            table.array_names(b"a").unwrap(),
            [
                b"k00".to_vec(),
                b"k01".to_vec(),
                b"k02".to_vec(),
                b"k03".to_vec()
            ]
        );
    }
    assert_eq!(crate::counters::finalize(), 0);
}

#[test]
fn declared_undefined_namespace_cells_remain_visible_without_becoming_defined() {
    // Native proof: naming.tcloo.explicit-variable-link-counted-target
    // docs/design/analysis/name-resolution-proofs/explicit-variable-link-counted-target.md
    let mut table = VarTable::default();
    let key = b"k\0tail";
    let alias = table.retain_native_scalar_alias(key);
    assert!(table.names().is_empty());
    table.mark_namespace_declared(key);
    drop(alias);
    assert_eq!(table.names(), [key.as_slice()]);
    assert!(table.non_link_names().is_empty());
    assert!(!table.is_set(key));
    assert!(table.load_scalar(key).is_none());
    assert!(table.has_native_namespace_cell(key));
    table.remove(key);
    assert!(table.names().is_empty());
    assert!(!table.is_set(key));
}

#[test]
fn trace_member_identity_selects_original_duplicate_root_allocations() {
    // Rust API coverage: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    crate::counters::reset();
    {
        let mut table = VarTable::default();
        let first = table.declare_compiled_cell(b"same");
        let second = table.declare_compiled_cell(b"same");
        let value = obj::Owned::fresh(obj::new_wide_int_obj(1));
        let mut members = Vec::new();
        for slot in [first, second] {
            let receiver = table
                .capture_receiver_at(slot, Some(b"k".to_vec()), true)
                .unwrap()
                .unwrap();
            receiver.store(value.as_ptr()).unwrap();
            let root = receiver.binding_id().unwrap();
            let member = table.native_trace_element_identity(root, b"k").unwrap();
            members.push((root, member));
        }
        assert_ne!(members[0].0, members[1].0);
        assert_ne!(members[0].1, members[1].1);
        for (root, member) in &members {
            assert_eq!(
                table.native_trace_element_identity(*root, b"k"),
                Some(*member)
            );
            assert_eq!(table.native_trace_element_identity(*root, b"absent"), None);
        }
        // A selected old member keeps its trace guard while a current
        // same-key replacement has an independent member allocation.
        let receiver = table
            .capture_receiver_at(first, None, true)
            .unwrap()
            .unwrap();
        let detached = receiver.begin_array_destruction().unwrap();
        assert_eq!(detached.trace_member_identity(b"k"), Some(members[0].1));
        receiver.ensure_array().unwrap();
        let replacement = table
            .capture_receiver_at(first, Some(b"k".to_vec()), true)
            .unwrap()
            .unwrap();
        replacement.store(value.as_ptr()).unwrap();
        let (_, replacement_id) = replacement.trace_member().unwrap();
        assert_ne!(replacement_id, members[0].1);
        assert_eq!(
            table.native_trace_element_identity(members[0].0, b"k"),
            Some(replacement_id)
        );
        let old = detached.capture_receiver(b"k".to_vec());
        assert_eq!(old.trace_member(), Some((b"k".to_vec(), members[0].1)));
        assert_eq!(old.read().unwrap(), Some(value.as_ptr()));
        detached.finish_destruction();
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

#[test]
fn c_declared_cells_preserve_duplicates_before_dynamic_hash_inventory() {
    crate::counters::reset();
    {
        let mut frames = FrameStack::new();
        frames.variable_hash_recipe = Some(NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Unsigned,
            width: NativeHashWordWidth::Bits32,
        });
        frames.push(crate::namespace::GLOBAL);
        frames.install_formal_cells(
            &[b"x".to_vec(), b"x".to_vec()],
            tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
                tcl_registry::InvocationDialect::for_version(TclVersion::V9_0)
                    .execution_point()
                    .unwrap(),
            )
            .unwrap(),
        );
        let one = obj::Owned::fresh(obj::new_wide_int_obj(1));
        let two = obj::Owned::fresh(obj::new_wide_int_obj(2));
        frames.store_formal_cell(0, one.as_ptr()).unwrap();
        frames.store_formal_cell(1, two.as_ptr()).unwrap();
        frames
            .table_mut(1)
            .unwrap()
            .store_scalar(b"extra", one.as_ptr())
            .unwrap();
        assert_eq!(
            frames.local_names(),
            [b"x".to_vec(), b"x".to_vec(), b"extra".to_vec()]
        );
        assert_eq!(
            frames.table(1).unwrap().load_scalar(b"x"),
            Some(one.as_ptr())
        );
        assert_eq!(
            frames
                .table(1)
                .unwrap()
                .entry_order
                .borrow()
                .physical_keys(),
            [b"extra".as_slice()]
        );
        assert!(
            matches!(frames.compiled_slot_var(1).as_deref(), Some(Var::Scalar(value)) if *value == two.as_ptr())
        );
    }
    assert_eq!(crate::counters::finalize(), 0);
}
