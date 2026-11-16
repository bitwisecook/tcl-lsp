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

//! TIP 380 slots: the built-in slot objects, the `::oo::Slot` base class, variable slot operations and declared-variable headers.

use super::*;

#[test]
fn variable_declaration_introspection_retains_original_counted_object() {
    // Native proof naming.tcloo.original-counted-variable-header:
    // docs/design/analysis/name-resolution-proofs/tcloo-original-counted-variable-header.md
    // Direct declaration/query retains the original counted input object;
    // byte-equal reconstructed values cannot supply this header identity.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"oo::class create C"), Code::Ok);
        let original = obj::Owned::fresh(obj::new_string_bytes(b"a\0z"));
        let mut declaration = vec![
            obj::Owned::fresh(obj::new_string_bytes(b"oo::define")),
            obj::Owned::fresh(obj::new_string_bytes(b"C")),
            obj::Owned::fresh(obj::new_string_bytes(b"variable")),
            original.clone(),
        ];
        let words: Vec<_> = declaration.iter().map(obj::Owned::as_ptr).collect();
        assert_eq!(interp.dispatch(&words), Code::Ok);
        declaration.clear();
        let query = [
            obj::Owned::fresh(obj::new_string_bytes(b"info")),
            obj::Owned::fresh(obj::new_string_bytes(b"class")),
            obj::Owned::fresh(obj::new_string_bytes(b"variables")),
            obj::Owned::fresh(obj::new_string_bytes(b"C")),
        ];
        let words: Vec<_> = query.iter().map(obj::Owned::as_ptr).collect();
        assert_eq!(interp.dispatch(&words), Code::Ok);
        let listed = list::list_elements(interp.result_obj()).unwrap();
        assert_eq!(listed, vec![original.as_ptr()], "{version}");
        assert_eq!(obj_bytes(listed[0]), b"a\0z", "{version}");
    }
}

#[test]
fn unknown_method_preserves_original_selector_for_returned_slot_data() {
    // naming.tcloo.original-counted-variable-header
    // docs/design/analysis/name-resolution-proofs/tcloo-original-counted-variable-header.md
    // This Rust control isolates the dispatcher before the native proof's
    // direct declaration/query boundary; it claims no extra native observer.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"oo::object create O; oo::objdefine O method unknown {requested args} {return $requested}"), Code::Ok);
        let selector = obj::Owned::fresh(obj::new_string_bytes(b"a\0z"));
        let head = obj::Owned::fresh(obj::new_string_bytes(b"O"));
        assert_eq!(
            interp.dispatch(&[head.as_ptr(), selector.as_ptr()]),
            Code::Ok,
            "{version}"
        );
        assert!(
            interp.result_obj() == selector.as_ptr(),
            "{version}: original selector header"
        );
        assert_eq!(interp.result_bytes(), b"a\0z", "{version}");
    }
}

#[test]
fn variable_slot_operations() {
    leak_free(|i| {
        // `variable` accumulates by default (-append), uniquely.
        ok(i, b"oo::class create A { variable a; variable b }");
        assert_eq!(ok(i, b"lsort [info class variables A]"), b"a b");
        // `-clear` empties; a following declaration starts fresh (oo-27.16).
        ok(
            i,
            b"oo::class create B { variable x; variable -clear; variable y }",
        );
        assert_eq!(ok(i, b"info class variables B"), b"y");
        // `-set` replaces (oo-27.17).
        ok(i, b"oo::class create C { variable x; variable -set y }");
        assert_eq!(ok(i, b"info class variables C"), b"y");
        // An unknown slot op errors with the C unknown-method message
        // (oo-27.18).
        assert_eq!(
            i.eval_str(b"oo::class create D { variable -? y }"),
            Code::Error
        );
        assert_eq!(
                i.result_bytes(),
                b"unknown method \"-?\": must be -append, -appendifnew, -clear, -prepend, -remove or -set"
            );
    });
}

#[test]
fn oo_slot_base_class() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create SampleSlot {\n\
                    superclass oo::Slot\n\
                    constructor {} { variable contents {a b c} }\n\
                    method contents {} { variable contents; return $contents }\n\
                    method Get {} { variable contents; return $contents }\n\
                    method Set {lst} { variable contents $lst; return }\n\
                    method Resolve {x} { return $x }\n\
                }",
        );
        ok(i, b"SampleSlot create s");
        ok(i, b"s -append g h");
        assert_eq!(ok(i, b"s contents"), b"a b c g h");
        ok(i, b"s -set d e");
        assert_eq!(ok(i, b"s contents"), b"d e");
        ok(i, b"s -prepend x");
        assert_eq!(ok(i, b"s contents"), b"x d e");
        ok(i, b"s -remove d");
        assert_eq!(ok(i, b"s contents"), b"x e");
        ok(i, b"s -clear");
        assert_eq!(ok(i, b"s contents"), b"");
        // Defaulting: a bare value goes through `unknown` → -append.
        ok(i, b"s p q");
        assert_eq!(ok(i, b"s contents"), b"p q");
    });
}

#[test]
fn builtin_slot_objects_present() {
    leak_free(|i| {
        // The eleven built-in slots are ::oo::Slot instances.
        assert_eq!(
            ok(i, b"lsort [info class instances ::oo::Slot]"),
            b"::oo::configuresupport::objreadableproperties \
                  ::oo::configuresupport::objwritableproperties \
                  ::oo::configuresupport::readableproperties \
                  ::oo::configuresupport::writableproperties ::oo::define::filter \
                  ::oo::define::mixin ::oo::define::superclass ::oo::define::variable \
                  ::oo::objdefine::filter ::oo::objdefine::mixin ::oo::objdefine::variable"
        );
        // filter slot: public ops from the class, Get/Set private.
        assert_eq!(
            ok(i, b"lsort [info object methods ::oo::define::filter -all]"),
            b"-append -appendifnew -clear -prepend -remove -set"
        );
        assert_eq!(
            ok(
                i,
                b"lsort [info object methods ::oo::define::filter -private]"
            ),
            b"Get Set"
        );
        // mixin slot also has a Resolve + a --default-operation forward.
        assert_eq!(
            ok(
                i,
                b"lsort [info object methods ::oo::define::mixin -private]"
            ),
            b"--default-operation Get Resolve Set"
        );
    });
}
