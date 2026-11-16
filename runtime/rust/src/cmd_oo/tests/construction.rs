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

//! Creating and copying objects and classes: constructors, metaclasses, `createWithNamespace`, the published object namespace and `oo::copy`.

use super::*;

#[test]
fn object_deleted_in_constructor() {
    leak_free(|i| {
        // Destroying the object in its constructor makes `new`/`create` fail
        // (oo-30.1/30.2, Bug 2903011), via `[self] destroy` or `my destroy`.
        ok(
            i,
            b"oo::class create cls { constructor {} {[self] destroy} }",
        );
        assert_eq!(i.eval_str(b"cls new"), Code::Error);
        assert_eq!(i.result_bytes(), b"object deleted in constructor");
        ok(i, b"oo::class create cls2 { constructor {} {my destroy} }");
        assert_eq!(i.eval_str(b"cls2 create foo"), Code::Error);
        assert_eq!(i.result_bytes(), b"object deleted in constructor");
    });
}

#[test]
fn oo_copy_clones_namespace_contents() {
    leak_free(|i| {
        // oo::copy copies the instance namespace's procs (re-pointed to the
        // copy's namespace) and variable values (oo-15.6/15.8).
        ok(i, b"oo::class create C { export eval }");
        ok(i, b"C create a");
        ok(
            i,
            b"a eval {variable y 0; proc foo {} {variable y; incr y}}",
        );
        assert_eq!(ok(i, b"a eval foo"), b"1"); // a.y = 1
        ok(i, b"oo::copy a b");
        // b inherits y=1 and its own foo (operating on b's y).
        assert_eq!(ok(i, b"b eval foo"), b"2"); // b.y 1->2
        assert_eq!(ok(i, b"b eval foo"), b"3"); // b.y 2->3
                                                // a's y is untouched by b's foo.
        assert_eq!(ok(i, b"a eval {set y}"), b"1");
        assert_eq!(ok(i, b"a eval foo"), b"2"); // a.y 1->2
    });
}

#[test]
fn metaclass_instantiation() {
    leak_free(|i| {
        // A metaclass's instances are themselves classes.
        ok(i, b"oo::class create meta { superclass oo::class }");
        ok(i, b"meta create instance1");
        ok(i, b"instance1 create instance2");
        assert_eq!(ok(i, b"info object class instance1"), b"::meta");
        assert_eq!(ok(i, b"info object class instance2"), b"::instance1");
        assert_eq!(ok(i, b"info object isa class instance1"), b"1");
        assert_eq!(ok(i, b"info object isa metaclass meta"), b"1");
        assert_eq!(ok(i, b"info object isa metaclass instance1"), b"0");
        assert_eq!(ok(i, b"info object isa object instance2"), b"1");
        // A metaclass instance can carry a definition-script body.
        ok(i, b"set c [meta create C { method foo {} { return MF } }]");
        assert_eq!(ok(i, b"[C new] foo"), b"MF");
        // Destroying the metaclass cascades to its class-instances (both maps
        // freed), so the names can be recreated.
        ok(i, b"meta destroy");
        assert_eq!(i.eval_str(b"instance1 create x"), Code::Error);
        ok(i, b"oo::class create instance1");
    });
}

#[test]
fn object_namespace_publication_uses_selected_relative_byte_geometry() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        leak_free(|i| {
            i.set_runtime_version(version);
            ok(
                i,
                b"namespace eval ::outer {oo::object create src; oo::copy src dst ns::::child}",
            );
            assert_eq!(
                ok(i, b"info object namespace ::outer::dst"),
                b"::outer::ns::child"
            );
            assert_eq!(
                i.eval_str(b"namespace eval ::outer {oo::copy src another ns::::child}"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"ns::::child refers to an existing namespace"
            );
            ok(i, b"namespace eval ::outer {oo::class create C; oo::objdefine C export createWithNamespace; C createWithNamespace named allocated::::child}");
            assert_eq!(
                ok(i, b"info object namespace ::outer::named"),
                b"::outer::allocated::child"
            );
            assert_eq!(
                i.eval_str(
                    b"namespace eval ::outer {C createWithNamespace another allocated::::child}"
                ),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"can't create namespace \"allocated::::child\": already exists"
            );
        });
    }
}

#[test]
fn create_with_namespace_conflicts_use_the_shared_native_report() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        for (seed, operand) in [
            (b"allocated".as_slice(), b"allocated".as_slice()),
            (b"allocated", b"::Outer::allocated"),
            (b"allocated", b"::::Outer::::allocated"),
            (b"allocated:tail", b"allocated:tail"),
        ] {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let source = [
                br#"namespace eval ::Outer {
    oo::class create C
    oo::objdefine C export createWithNamespace
    namespace eval "#
                    .as_slice(),
                seed,
                b" {}\nset operand ",
                operand,
                br#"
    set status [catch {C createWithNamespace object $operand} value]
    list $status $value $::errorCode [info commands object]
}"#,
            ]
            .concat();
            assert_eq!(interpreter.eval_str(&source), Code::Ok);
            let expected = [
                b"1 {can't create namespace \"".as_slice(),
                operand,
                b"\": already exists} {TCL OPERATION NAMESPACE CREATEEXISTING} {}",
            ]
            .concat();
            assert_eq!(interpreter.result_bytes(), expected, "{version:?}");
            assert!(interpreter.native_execution_refusal().is_none());
        }
    }
}

#[test]
fn create_with_namespace_and_copy_targetns() {
    leak_free(|i| {
        // `createWithNamespace` is unexported by default; once exported it
        // creates the object in a fresh (must-not-exist) namespace.
        ok(i, b"oo::class create AC");
        assert_eq!(
            i.eval_str(b"AC createWithNamespace inst ::ns1"),
            Code::Error
        );
        ok(i, b"oo::objdefine AC export createWithNamespace");
        ok(i, b"AC createWithNamespace inst ::ns1");
        assert_eq!(ok(i, b"info object namespace inst"), b"::ns1");
        // An already-existing namespace is rejected.
        ok(i, b"namespace eval ::taken {}");
        assert_eq!(
            i.eval_str(b"AC createWithNamespace inst2 ::taken"),
            Code::Error
        );
        // `oo::copy src tgt targetNamespace` places the copy's vars there.
        ok(i, b"oo::class create C { method m {} {} }");
        ok(i, b"C create src");
        ok(i, b"oo::copy src dst ::dstns");
        assert_eq!(ok(i, b"info object namespace dst"), b"::dstns");
    });
}

#[test]
fn oo_copy_clones_classes() {
    leak_free(|i| {
        // `oo::copy` of a class clones the class facet, so the copy is a
        // working class whose instances run the cloned methods.
        ok(
            i,
            b"oo::class create foo { method testme {} { return [self class] } }",
        );
        ok(i, b"oo::copy foo bar");
        assert_eq!(ok(i, b"[bar new] testme"), b"::bar");
        // Cloned superclasses + declared variables come across too.
        ok(i, b"oo::class create AC");
        ok(i, b"oo::class create Foo { superclass AC; variable a b c }");
        ok(i, b"oo::copy Foo Bar");
        assert_eq!(ok(i, b"info class variable Bar"), b"a b c");
        assert_eq!(ok(i, b"info class superclasses Bar"), b"::AC");
    });
}

#[test]
fn my_create_reaches_class_factory() {
    leak_free(|i| {
        // A metaclass method can instantiate via `my create`, even when the
        // class unexports `create` for external callers (TclOO factory).
        ok(
            i,
            b"oo::class create meta { superclass oo::class; self { unexport create new; \
                   method make {x} { my create $x } } }",
        );
        // External create is hidden (unknown method, not listing create/new).
        assert_eq!(i.eval_str(b"meta create foo"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown method \"create\": must be destroy or make"
        );
        // The internal `my create` still works.
        assert_eq!(ok(i, b"meta make ::bar"), b"::bar");
        assert_eq!(ok(i, b"info object class ::bar"), b"::meta");
    });
}
