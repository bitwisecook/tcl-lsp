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

mod definition_exit;

mod tests {
    use crate::prelude::*;

    fn vm(version: tcl_dialect::TclVersion) -> Vm {
        let profile =
            tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                .unit_profile();
        crate::fixture::interpreter(profile)
    }

    #[test]
    fn helper_cpp_executes_only_the_actual_method_frame_and_original_words() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result=vm.eval_source(r"oo::class create B {method m args {list BASE {*}$args}; method by args {list BASE {*}$args}; method static args {list STATIC {*}$args}}
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
").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str().as_ref()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "{BASE A B} {BASE C D} {STATIC ONE} ::obj 1 1 0",
                "{version:?}"
            );
            let nested = vm.eval_source("obj nested").unwrap();
            assert_eq!(nested.code, Code::Error);
            assert_eq!(
                nested.options.as_list().unwrap()[1].to_str().as_ref(),
                "TCL OO CONTEXT_REQUIRED"
            );
            let evaluated = vm
                .eval_source(
                    "oo::define C method evalContext {} {my eval {self object}}; obj evalContext",
                )
                .unwrap();
            assert_eq!(evaluated.code, Code::Ok);
            assert_eq!(evaluated.result.to_str().as_ref(), "::obj");
        }
    }

    #[test]
    fn definition_namespace_and_variable_activation_are_original() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result = vm.eval_source("proc ::method args {error GLOBAL_WORKER}; namespace eval caller {proc build {} {set private CALLER; oo::class create ::C {set scope [namespace current]; set inherited [info exists private]; set local [uplevel 1 {set private}]; proc born {} {return HERE}; meth m {} {return OK}}}}; caller::build; C create obj; list [obj m] $::oo::define::scope $::oo::define::inherited $::oo::define::local [::oo::define::born]").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str().as_ref()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "OK ::oo::define 0 CALLER HERE"
            );
            let outside = vm.invoke_command(
                "::oo::define::method",
                &[Value::string("m"), Value::empty(), Value::empty()],
            );
            assert_eq!(outside.code, Code::Error);
            assert_eq!(
                outside.options.as_list().unwrap()[1].to_str().as_ref(),
                "TCL OO MONKEY_BUSINESS"
            );
            assert_eq!(
                vm.try_eval_source("namespace current")
                    .unwrap()
                    .result
                    .to_str()
                    .as_ref(),
                "::"
            );
        }
    }

    #[test]
    fn nested_definitions_retain_actual_caller_frames_and_qualified_workers() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result = vm.eval_source(r"namespace eval ::outer {
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
list $chained $ordinary $magic [namespace current] $outside [dict get $options -errorcode]").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str().as_ref()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "{::outer::B OUTER} ::oo::define::B {{::oo::define::inspect FIRST SECOND} {::oo::define::inspect THIRD}} :: 1 {TCL OO MONKEY_BUSINESS}"
            );
        }
    }

    #[test]
    fn cached_object_name_retains_the_original_header_until_rename() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            assert_eq!(vm.eval_source("oo::class create C {method object {} {self object}; method nested {} {p}; method shifted {} {uplevel #0 {::oo::Helpers::self}}}; proc p {} {::oo::Helpers::self}; C create obj").unwrap().code, Code::Ok);
            let first = vm.invoke_command("obj", &[Value::string("object")]);
            assert_eq!(first.code, Code::Ok);
            // The native observer retains Tcl_GetObjResult across later calls.
            // Completion itself supplies only a lifetime view of that role.
            let first = first.result.into_native_reference();
            first.as_list().unwrap();
            let second = vm.invoke_command("obj", &[Value::string("object")]);
            assert_eq!(second.code, Code::Ok);
            assert_eq!(
                first.native_object_identity(),
                second.result.native_object_identity()
            );
            assert_eq!(
                vm.invoke_command("obj", &[Value::string("nested")]).code,
                Code::Error
            );
            assert_eq!(
                vm.invoke_command("obj", &[Value::string("shifted")]).code,
                Code::Error
            );
            assert_eq!(
                vm.invoke_command("rename", &[Value::string("obj"), Value::string("renamed")])
                    .code,
                Code::Ok
            );
            let renamed = vm.invoke_command("renamed", &[Value::string("object")]);
            assert_eq!(renamed.code, Code::Ok);
            let renamed = renamed.result.into_native_reference();
            assert_ne!(
                first.native_object_identity(),
                renamed.native_object_identity()
            );
            let deleted = vm
                .eval_source(
                    "oo::define C method deleted {} {rename [self] {}; self}; renamed deleted",
                )
                .unwrap();
            assert_eq!(deleted.code, Code::Ok);
            assert_eq!(deleted.result.to_str().as_ref(), "");
            assert_eq!(first.to_str().as_ref(), "::obj");
            assert_eq!(renamed.to_str().as_ref(), "::renamed");
        }
    }

    #[test]
    fn uplevel_selection_restores_original_namespace_for_source_and_object_bodies() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = vm(version);
            let result = vm.eval_source("namespace eval ::caller {proc parent {} {set private CALLER; set got [namespace eval ::child {set fromScript [uplevel 1 {set private}]; set fromList [uplevel 1 [list set private]]; list $fromScript $fromList [namespace current]}]; list $got [namespace current]}; parent}").unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{version:?}: {}",
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                "{CALLER CALLER ::child} ::caller"
            );
            assert_eq!(
                vm.try_eval_source("namespace current")
                    .unwrap()
                    .result
                    .to_str()
                    .as_ref(),
                "::"
            );
        }
    }
}
