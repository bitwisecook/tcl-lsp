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
use crate::interp::obj_bytes;

#[test]
fn definition_head_uses_current_constructed_namespace_and_selected_tail() {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.0"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interp.eval_str(b"proc choice {} {}; namespace eval N {proc literal: {} {}; proc choice {} {}; proc chosen {} {}}"), Code::Ok);
    let root = interp.oo_definition_head(b"choice").unwrap();
    assert_eq!(obj::bytes_of(root.as_ptr()), b"::choice");
    let namespace = interp.find_namespace_id(b"::N").unwrap();
    interp.set_current_ns(namespace);
    let exact = interp.oo_definition_head(b"choice").unwrap();
    assert_eq!(obj::bytes_of(exact.as_ptr()), b"::N::choice");
    let literal = interp.oo_definition_head(b"lit").unwrap();
    assert_eq!(obj::bytes_of(literal.as_ptr()), b"::N::literal:");
    assert!(interp.oo_definition_head(b"cho").is_none());
    assert!(interp.oo_definition_head(b"").is_none());
    assert!(interp.oo_definition_head(b"::choice").is_none());
    interp.set_current_ns(crate::namespace::GLOBAL);
    let restored = interp.oo_definition_head(b"choice").unwrap();
    assert_eq!(obj::bytes_of(restored.as_ptr()), b"::choice");
    assert!(!interp.host_refusal_pending());
}

#[test]
fn self_call_uses_the_selected_uplevel_activation_and_restores_its_caller() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(
                br"oo::class create Outer {method run {peer} {$peer inspect}}
oo::class create Inner {method inspect {} {
    set direct [self call]
    set caller [uplevel 1 {::oo::Helpers::self call}]
    list $direct $caller [self call]
}}
Outer create outer
Inner create inner
outer run inner"
            ),
            Code::Ok,
            "{version}: {:?}",
            interp.result_bytes()
        );
        assert_eq!(
                interp.result_bytes(),
                b"{{{method inspect ::Inner method}} 0} {{{method run ::Outer method}} 0} {{{method inspect ::Inner method}} 0}",
                "{version}"
            );
        assert_eq!(
            interp.eval_str(b"proc ordinary {} {::oo::Helpers::self call}; ordinary"),
            Code::Error,
            "{version}"
        );
        assert_eq!(interp.error_code(), b"TCL OO CONTEXT_REQUIRED", "{version}");
        assert_eq!(
            interp.result_bytes(),
            b"::oo::Helpers::self may only be called from inside a method",
            "{version}"
        );
        assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
        assert_eq!(interp.error_code(), b"NONE", "{version}");
    }
}

#[test]
fn dynamic_self_subcommands_use_the_original_native_index_table() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
                interp.eval_str(b"oo::class create C {method query {which} {self $which}}; C create obj; obj query o"),
                Code::Ok
            );
        assert_eq!(interp.result_bytes(), b"::obj");
        assert_eq!(
            interp.eval_str(b"string match ::oo::Obj* [obj query nam]"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"1");
        for (script, error_code) in [
            (
                b"obj query c".as_slice(),
                b"TCL LOOKUP INDEX subcommand c".as_slice(),
            ),
            (b"obj query n", b"TCL LOOKUP INDEX subcommand n"),
            (b"obj query {}", b"TCL LOOKUP INDEX subcommand {}"),
            (b"obj query missing", b"TCL LOOKUP INDEX subcommand missing"),
        ] {
            assert_eq!(
                interp.eval_str(script),
                Code::Error,
                "{version}: {script:?}"
            );
            assert_eq!(interp.error_code(), error_code, "{version}: {script:?}");
        }
    }
}

#[test]
fn helper_artifact_executes_only_the_actual_method_frame_and_original_words() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        assert_eq!(interp.eval_str(br"oo::class create B {method m args {list BASE {*}$args}; method by args {list BASE {*}$args}; method static args {list STATIC {*}$args}}
oo::class create C {
 superclass B
 method m args {next {*}$args}
 method by args {nextto B {*}$args}
 method static {} {nextto {*}{B ONE}}
 method object {} {self o}
 method space {} {self n}
 method nested {} {ordinary}
}
proc ordinary {} {::oo::Helpers::self}
C create obj
list [obj m A B] [obj by C D] [obj static] [obj object] [string match ::oo::Obj* [obj space]] [catch {obj nested}] [llength [info commands ::self]]
"),Code::Ok,"{version}: {:?}",interp.result_bytes());
        assert_eq!(
            interp.result_bytes(),
            b"{BASE A B} {BASE C D} {STATIC ONE} ::obj 1 1 0",
            "{version}"
        );
        assert_eq!(interp.eval_str(b"obj nested"), Code::Error);
        assert_eq!(interp.error_code(), b"TCL OO CONTEXT_REQUIRED");
        assert_eq!(
            interp.eval_str(
                b"oo::define C method evalContext {} {my eval {self object}}; obj evalContext"
            ),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"::obj");
    }
}

#[test]
fn definition_namespace_owns_its_scope_and_original_workers() {
    let mut interp = Interp::new();
    let script = b"proc ::method args {error GLOBAL_WORKER}; namespace eval caller {proc build {} {set private CALLER; oo::class create ::C {set scope [namespace current]; set inherited [info exists private]; set local [uplevel 1 {set private}]; proc born {} {return HERE}; meth m {} {return OK}}}}; caller::build; C create obj; list [obj m] $::oo::define::scope $::oo::define::inherited $::oo::define::local [::oo::define::born]";
    assert_eq!(
        interp.eval_str(script),
        Code::Ok,
        "{:?}",
        interp.result_bytes()
    );
    assert_eq!(interp.result_bytes(), b"OK ::oo::define 0 CALLER HERE");
    assert_eq!(
        interp.eval_str(b"::oo::define::method m {} {}"),
        Code::Error
    );
    assert_eq!(interp.error_code(), b"TCL OO MONKEY_BUSINESS");
    assert_eq!(interp.current_ns(), GLOBAL);
}

#[test]
fn nested_definitions_retain_actual_caller_frames_and_qualified_workers() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        assert_eq!(interp.eval_str(br"namespace eval ::outer {
 oo::class create B {}
 oo::class create C {}
 oo::class create D {}
 proc build {} {
  oo::define C {oo::define ::outer::D {superclass B; uplevel 1 {::oo::define::method resumed {} {return OUTER}}}}
 }
 build
}
::outer::C create obj
set chained [list [info class superclasses ::outer::D] [obj resumed]]
oo::class create ::oo::define::B {}
proc ::oo::define::bridge {} {oo::define ::outer::D {superclass B}}
oo::define ::outer::C {bridge}
set ordinary [info class superclasses ::outer::D]
proc ::inspect args {error GLOBAL}
proc ::oo::define::inspect args {list [lindex [info level 0] 0] {*}$args}
set magic [list [oo::define ::outer::C insp FIRST SECOND] [oo::define ::outer::C inspect THIRD]]
set outside [catch {::oo::define::method outside {} {}} result options]
list $chained $ordinary $magic [namespace current] $outside [dict get $options -errorcode]"), Code::Ok, "{version}: {:?}", interp.result_bytes());
        assert_eq!(interp.result_bytes(), b"{::outer::B OUTER} ::oo::define::B {{::oo::define::inspect FIRST SECOND} {::oo::define::inspect THIRD}} :: 1 {TCL OO MONKEY_BUSINESS}");
        assert!(interp.oo.borrow().def_stack.is_empty());
    }
}

#[test]
fn cached_self_name_is_the_same_header_and_nested_procs_have_no_context() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"oo::class create C {method object {} {self object}; method nested {} {p}; method shifted {} {uplevel #0 {::oo::Helpers::self}}}; proc p {} {::oo::Helpers::self}; C create obj; obj object"), Code::Ok);
    let original = obj::Owned::retain(interp.get_obj_result());
    list::list_elements(original.as_ptr()).unwrap();
    assert_eq!(interp.eval_str(b"obj object"), Code::Ok);
    assert_eq!(original.as_ptr(), interp.get_obj_result());
    assert!(core::ptr::eq(
        obj::obj_type_ptr(interp.get_obj_result()),
        &list::TCL_LIST_TYPE
    ));
    assert_eq!(interp.eval_str(b"obj nested"), Code::Error);
    assert_eq!(interp.eval_str(b"obj shifted"), Code::Error);
    assert_eq!(
        interp.eval_str(b"rename obj renamed; renamed object"),
        Code::Ok
    );
    assert_ne!(original.as_ptr(), interp.get_obj_result());
    let renamed = obj::Owned::retain(interp.get_obj_result());
    assert_eq!(
        interp
            .eval_str(b"oo::define C method deleted {} {rename [self] {}; self}; renamed deleted"),
        Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"");
    assert_eq!(obj_bytes(original.as_ptr()), b"::obj");
    assert_eq!(obj_bytes(renamed.as_ptr()), b"::renamed");
}
