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
use crate::cmd_oo::Method;
fn instance() -> Interp {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interp.eval_str(b"oo::configurable create C {property yellow -get {return Y}; property zinc -get {return Z}}; C create o"),Code::Ok,"{:?}",interp.result_bytes());
    interp
}
#[test]
fn property_headers_and_temporary_tables_retain_original_members() {
    let mut interp = instance();
    let id = interp.oo_resolve_object(b"o");
    let header = interp.native_all_property_header(id, false, false).unwrap();
    let strings = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .unwrap()
        .strings();
    let members = crate::list::list_elements_native_checked(header.header, strings).unwrap();
    let first = members[0];
    let refs = unsafe { (*first).ref_count };
    let copy = crate::list::native_list_copy(header.header, strings).unwrap();
    let copied = crate::list::list_elements_native_checked(copy.as_ptr(), strings).unwrap();
    assert_ne!(copy.as_ptr(), header.header);
    assert_eq!(copied[0], first);
    assert_eq!(unsafe { (*first).ref_count }, refs);
    let caller = obj::Owned::fresh(obj::new_string_bytes(b"-y"));
    let mut table = None;
    assert_eq!(
        {
            let selected =
                lookup_property(&mut interp, id, caller.as_ptr(), false, &mut table, true).unwrap();
            interp.native_object_string_bytes(selected.header).unwrap()
        }
        .as_ref(),
        b"-yellow"
    );
    assert_eq!(
        obj::native_object_snapshot(caller.as_ptr()).unwrap().cache,
        tcl_syntax::native_object::NativeObjectCacheSnapshot::None
    );
    assert!(table.is_some());
    assert_eq!(
        interp
            .native_all_property_header(id, false, false)
            .unwrap()
            .header,
        header.header
    );
}
#[test]
fn opaque_property_lookup_uses_original_accessor_members() {
    // Native proof: naming.property.opaque-name-custom-getter-option-cache
    // docs/design/analysis/name-resolution-proofs/property-opaque-name-custom-getter-option-cache.md
    let mut interp = instance();
    let target = interp.oo_resolve_object(b"o");
    for name in [b"x\xff".as_slice(), b"x\0tail".as_slice()] {
        let words = [
            b"oo::define".as_slice(),
            b"C",
            b"property",
            name,
            b"-get",
            b"return RAW",
        ];
        let owners: Vec<_> = words
            .into_iter()
            .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
            .collect();
        let argv: Vec<_> = owners.iter().map(obj::Owned::as_ptr).collect();
        assert_eq!(interp.dispatch(&argv), Code::Ok);
        let mut dashed = b"-".to_vec();
        dashed.extend_from_slice(name);
        let caller = obj::Owned::fresh(obj::new_string_bytes(&dashed));
        assert_eq!(
            configure_native(&mut interp, target, &[caller.as_ptr()]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"RAW");
        assert_eq!(
            obj::native_object_snapshot(caller.as_ptr()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
        let mut table = None;
        let selected = lookup_property(
            &mut interp,
            target,
            caller.as_ptr(),
            false,
            &mut table,
            false,
        )
        .unwrap();
        assert!(obj::native_property_name::is_cached(selected.header));
    }
    let rows = include_str!("../../../tests/data/native_property_opaque/native.tsv");
    assert_eq!(rows, "opaque-0\tRAW\tnone\nopaque-1\tRAW\tnone\n");
}
#[test]
fn default_property_methods_retain_original_clientdata_and_leave_its_primary() {
    // Native proof: naming.property.original-accessor-clientdata
    // docs/design/analysis/name-resolution-proofs/property-original-accessor-clientdata.md
    let mut interp = instance();
    let class = interp.oo_resolve_object(b"C");
    let target = interp.oo_resolve_object(b"o");
    let recipe = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .unwrap();
    for name in [b"p".as_slice(), b"x\xff".as_slice(), b"x\0tail".as_slice()] {
        let declaration = obj::Owned::fresh(obj::new_string_bytes(name));
        let prefix: Vec<_> = [b"oo::define".as_slice(), b"C", b"property"]
            .into_iter()
            .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
            .collect();
        let mut argv: Vec<_> = prefix.iter().map(obj::Owned::as_ptr).collect();
        argv.push(declaration.as_ptr());
        assert_eq!(
            interp.dispatch(&argv),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
        let mut canonical = b"-".to_vec();
        canonical.extend_from_slice(tcl_core_types::c_string_extent(name));
        let (reader, writer) = recipe.member_accessor_names(&canonical);
        for key in [reader, writer] {
            let method = interp.oo.borrow().classes[&class]
                .methods
                .get(key.as_slice())
                .unwrap();
            let Method::Property(property) = method else {
                panic!("native property method");
            };
            assert_eq!(property.original.as_ptr(), declaration.as_ptr());
        }
        assert_eq!(unsafe { (*declaration.as_ptr()).ref_count }, 3);
        let mut query = b"-".to_vec();
        query.extend_from_slice(name);
        let query = obj::Owned::fresh(obj::new_string_bytes(&query));
        let supplied = obj::Owned::fresh(obj::new_string_bytes(b"RAW"));
        assert_eq!(
            configure_native(&mut interp, target, &[query.as_ptr(), supplied.as_ptr()]),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
        assert!(interp.result_bytes().is_empty());
        assert_eq!(
            configure_native(&mut interp, target, &[query.as_ptr()]),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
        assert_eq!(interp.result_obj(), supplied.as_ptr());
        assert_eq!(unsafe { (*declaration.as_ptr()).ref_count }, 3);
        assert_eq!(
            obj::native_object_snapshot(declaration.as_ptr())
                .unwrap()
                .cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
        assert_eq!(
            obj::native_object_snapshot(query.as_ptr()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
    }
    let rows = include_str!("../../../tests/data/native_property_clientdata/native.tsv");
    assert_eq!(rows.lines().count(), 9);
    for index in 0..3 {
        assert!(rows.contains(&format!("decl-{index}\t1\t1\t3\tnone\n")));
        assert!(rows.contains(&format!("write-{index}\t3\tnone\t1\n")));
        assert!(rows.contains(&format!("read-{index}\t3\tnone\t1\tnone\n")));
    }
}
#[test]
fn property_epochs_follow_mutations_before_definition_failure() {
    // Native proof: naming.property.original-foundation-epoch-and-cache
    // docs/design/analysis/name-resolution-proofs/property-original-foundation-epoch-and-cache.md
    let mut interp = instance();
    let id = interp.oo_resolve_object(b"o");
    let class = interp.oo_resolve_object(b"C");
    let old = interp.native_all_property_header(id, false, false).unwrap();
    let epoch = interp.oo.borrow().property_foundation_epoch;
    interp.native_property_method_created(id, false).unwrap();
    assert_eq!(interp.oo.borrow().property_foundation_epoch, epoch);
    assert_eq!(
        interp
            .native_all_property_header(id, false, false)
            .unwrap()
            .header,
        old.header
    );
    assert_eq!(
        interp.eval_str(b"catch {oo::define C {::oo::define::method live {} {return};error LATE}}"),
        Code::Ok
    );
    assert_eq!(interp.oo.borrow().property_foundation_epoch, epoch + 1);
    assert_eq!(
        interp.oo.borrow().property_caches[&(id, false)]
            .readable
            .as_ref()
            .unwrap()
            .as_ptr(),
        old.header
    );
    assert_ne!(
        interp
            .native_all_property_header(id, false, false)
            .unwrap()
            .header,
        old.header
    );
    let epoch = interp.oo.borrow().property_foundation_epoch;
    interp
        .native_property_structure_changed(class, true)
        .unwrap();
    assert_eq!(interp.oo.borrow().property_foundation_epoch, epoch + 1);
}
