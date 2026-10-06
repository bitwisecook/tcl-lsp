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

//! Actual native compilation entry and command-selection timing, through the VM.

use std::path::Path;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_test_support::{available_tclshs, locate_jimsh, require_jimsh, run_script};
use tcl_vm::Vm;

const CASES: &[(&str, &str, Option<&str>)] = &[
    (
        "literal-before-argv",
        "set x [proc set {args} {return CUSTOM}]; return [info exists x]",
        None,
    ),
    (
        "dynamic-head-after-argv",
        "set c set; $c x [proc set {args} {return CUSTOM}]; return [info exists x]",
        None,
    ),
    (
        "alias-after-argv",
        "interp alias {} s {} set; s x [proc set {args} {return CUSTOM}]; return [info exists x]",
        Some("alias s set; s x [proc set {args} {return CUSTOM}]; return [info exists x]"),
    ),
    (
        "replacement-earlier-in-chunk",
        "proc set {args} {return CUSTOM}; set x value; return [info exists x]",
        None,
    ),
    (
        "break-chunk-entry",
        "set ::called 0; proc break {} {incr ::called}; set i 0; while {$i < 2} {incr i; break}; return [list $i $::called]",
        None,
    ),
    (
        "continue-chunk-entry",
        "set ::called 0; proc continue {} {incr ::called}; set i 0; while {$i < 2} {incr i; continue}; return [list $i $::called]",
        None,
    ),
    (
        "simple-return-before-argv",
        "return [proc return {args} {list CUSTOM} ]",
        None,
    ),
    (
        "protected-return-before-argv",
        "catch {return [proc return {args} {list CUSTOM}]} result; list $result",
        None,
    ),
];

#[test]
fn procedure_header_policy_matches_actual_native_compiler_registration() {
    let cases = [
        ("args", ""),
        (" args ", " \t\n\u{b}\u{c}\r"),
        ("{args}", ""),
        ("\targs", ""),
        ("args", "\\\n"),
        ("args", "# comment"),
        ("value", "return OK"),
        ("value", ""),
    ];
    for reference in available_tclshs() {
        let dialect = tcl_registry::InvocationDialect::for_version(reference.version);
        for (parameters, body) in cases {
            compare_native_procedure_header(&reference.path, dialect, parameters, body);
        }
    }
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("validated Jim override")
    };
    if let Some(jim) = jim {
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        for (parameters, body) in cases {
            compare_native_procedure_header(&jim.path, dialect, parameters, body);
        }
    }
}

#[test]
fn selected_noop_header_survives_argument_mutation_once() {
    let cases = [
        "proc noop args {}; set ::count 0; proc P {} {set value [noop [incr ::count; proc noop args {return CUSTOM}]]; list $value $::count}; P",
        "proc noop args {}; set ::count 0; proc P {} {set value \"[noop [incr ::count; proc noop args {return CUSTOM}]]\"; list $value $::count}; P",
        "proc noop args {}; set ::count 0; proc P {} {return [noop [incr ::count; proc noop args {return CUSTOM}]]}; list [P] $::count",
        "proc noop args {}; proc P {} {noop [rename noop {}; set ::changed 1]}; list [P] $::changed",
        "proc noop args {}; proc P {} {proc noop args {return CUSTOM}; noop}; P",
        "proc noop args {}; proc P {} {set c noop; $c [proc noop args {return CUSTOM}]}; P",
        "proc noop args {}; namespace eval N {proc P {} {proc noop args {return LOCAL}; noop}}; N::P",
        "proc noop args {}; proc observe args {incr ::traced}; set ::traced 0; proc P {} {noop [trace add execution noop enter observe]; set ::traced}; P",
        "proc noop args {}; proc observe args {incr ::traced}; set ::traced 0; trace add execution noop enter observe; proc P {} {noop; set ::traced}; P",
        "proc origin args {}; namespace export origin; namespace eval N {namespace import ::origin; proc P {} {origin [rename ::origin {}; set ::changed 1]}}; list [N::P] $::changed",
        "proc origin args {}; namespace export origin; namespace eval N {namespace import ::origin; proc P {} {origin [proc ::origin args {return CUSTOM}]}}; N::P",
    ];
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .unwrap()
            .strict_text()
            .unwrap();
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                result.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

#[test]
fn reused_handles_preserve_native_compiler_invalidation_policy() {
    let cases = [
        (
            "proc noop args {return OLD}; set ::count 0",
            "noop [incr ::count; if {$::count > 1} {rename noop {}}]",
            "proc noop args {}",
        ),
        (
            "proc noop args {}; set ::count 0",
            "noop [incr ::count; if {$::count > 1} {proc noop args {return CUSTOM}}]",
            "proc noop args {return OLD}",
        ),
        (
            "proc noop args {return OLD}; proc observed {} {set x 1}; proc observer args {return}; set ::count 0; trace add execution observed enter observer",
            "noop [incr ::count; if {$::count > 1} {rename noop {}}]",
            "proc noop args {}; rename observed {}",
        ),
        (
            "proc noop args {return OLD}; proc observed {} {set x 1}; proc observer args {return}; set ::count 0; trace add execution observed enterstep observer",
            "noop [incr ::count; if {$::count > 1} {rename noop {}}]",
            "proc noop args {}; rename observed {}",
        ),
    ];
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for (setup, body, mutation) in cases {
            // Reuse one natively compiled procedure body across the mutation.
            let native = run_script(
                &reference.path,
                format!("{setup}; proc P {{}} {{{body}}}; set first [P]; {mutation}; set code [catch {{P}} result]; puts [list $first $code $result $::count]\n").as_bytes(),
            ).unwrap().strict_text().unwrap();
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            assert!(vm.try_eval_source(setup).unwrap().code.is_ok());
            let handle = vm.try_compile_function(body).unwrap();
            let first = vm.try_invoke_function(&handle).unwrap();
            assert!(first.code.is_ok());
            assert!(vm.try_eval_source(mutation).unwrap().code.is_ok());
            let second = vm.try_invoke_function(&handle).unwrap();
            let count = vm.try_eval_source("set ::count").unwrap();
            let observed = tcl_syntax::list::join_list([
                first.result.to_str().as_ref(),
                &second.code.as_int().to_string(),
                second.result.to_str().as_ref(),
                count.result.to_str().as_ref(),
            ]);
            assert_eq!(observed, native, "{} {setup} / {mutation}", profile.name);
        }
    }
}

#[test]
fn namespace_resolver_cache_admission_matches_native_c() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let mut cases = vec![
            "proc noop args {}; namespace eval N {proc P {} {noop}}; set first [N::P]; proc N::noop args {return LOCAL}; list $first [N::P]",
            "proc noop args {return GLOBAL}; set ::count 0; namespace eval N {proc P {} {noop [incr ::count; if {$::count > 1} {rename ::N::noop {}}]}}; set first [N::P]; proc N::noop args {}; list $first [N::P] $::count",
            "namespace eval B {proc noop args {}}; namespace eval N {proc P {} {B::noop}}; set first [N::P]; namespace eval N::B {proc noop args {return LOCAL}}; list $first [N::P]",
            "namespace eval N {namespace export helper; proc oldnoop args {}; proc helper {} {oldnoop}; proc P {} {namespace delete ::N; namespace eval ::N {proc oldnoop args {return NEW}}; ::external}}; namespace import N::helper; proc external {} {helper}; N::P",
        ];
        if reference.version >= tcl_dialect::TclVersion::V8_5 {
            cases.push("set ::count 0; namespace eval A {proc noop args {return OLD}}; namespace eval B {proc noop args {}}; namespace eval N {namespace path ::A; proc P {} {noop [incr ::count; if {$::count > 1} {rename ::B::noop {}}]}}; set first [N::P]; namespace eval N {namespace path ::B}; list $first [N::P] $::count");
        }
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .unwrap()
            .strict_text()
            .unwrap();
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                result.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

fn compare_native_procedure_header(
    interpreter: &Path,
    dialect: tcl_registry::InvocationDialect,
    parameters: &str,
    body: &str,
) {
    let encode = |text: &str| {
        if text.is_empty() {
            "{}".to_owned()
        } else {
            text.as_bytes()
                .iter()
                .fold(String::new(), |mut result, byte| {
                    use std::fmt::Write;
                    write!(result, "{byte:02x}").expect("String writer");
                    result
                })
        }
    };
    let source = format!(
        "set formal [binary format H* {}]; set body [binary format H* {}]; proc noop $formal $body\nproc probe {{}} {{noop [rename noop {{}}; set ::changed 1]}}\nset code [catch {{probe}} result]\nbinary scan $result H* bytes\nputs [list $code $bytes $::changed]\n",
        encode(parameters),
        encode(body),
    );
    let native = run_script(interpreter, source.as_bytes())
        .unwrap()
        .strict_text()
        .unwrap();
    let header = tcl_registry::native_procedure::procedure_header_compilation(
        dialect,
        Some(parameters),
        Some(body),
        Some(false),
    );
    let expected = match header {
        tcl_dialect::NativeProcedureHeaderCompilation::NoOp => "0 {} 1",
        tcl_dialect::NativeProcedureHeaderCompilation::Absent => {
            "1 696e76616c696420636f6d6d616e64206e616d6520226e6f6f7022 1"
        }
        tcl_dialect::NativeProcedureHeaderCompilation::Unknown => {
            panic!("pinned native header policy")
        }
    };
    assert_eq!(
        native,
        expected,
        "{} {parameters:?} {body:?}",
        interpreter.display()
    );
}

#[test]
fn actual_variable_observers_preserve_compiled_selection_and_callback_effects() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let registration = if reference.version == tcl_dialect::TclVersion::V8_4 {
            "trace variable ::count w cb"
        } else {
            "trace add variable ::count write cb"
        };
        let source = format!(
            "proc cb args {{proc return args {{list CUSTOM}}}};set ::count 0;{registration};proc operand {{}} {{incr ::count;return X}};operand"
        );
        let native = run_script(
            &reference.path,
            format!("puts [eval {{{source}}}]\n").as_bytes(),
        )
        .unwrap()
        .strict_text()
        .unwrap();
        let mut vm = Vm::default();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let completion = vm
            .try_eval_source(&source)
            .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
        assert!(
            completion.code.is_ok(),
            "{}: {}",
            profile.name,
            completion.result.to_str()
        );
        assert_eq!(
            completion.result.to_str().as_ref(),
            native,
            "{} {source}",
            profile.name
        );
    }
}

#[test]
fn native_uplevel_selection_and_frames_match_actual_c() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in [
            "proc P {} {set x LOCAL;uplevel 0 {set x}};P",
            "proc P {} {set x LOCAL;uplevel #0 {set g GLOBAL};list $x $::g};P",
            "proc P {} {uplevel 0 set x 1;set x};P",
            "proc P {} {uplevel 0 [proc uplevel args {return REPLACED};list set x ORIGINAL]};P",
            "proc P {} {uplevel #0 {error BOOM}};set code [catch {P} result];list $code $result",
        ] {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .unwrap()
            .strict_text()
            .unwrap();
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let completion = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                completion.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                completion.result.to_str()
            );
            assert_eq!(
                completion.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

#[test]
fn actual_info_ensemble_private_commands_and_map_mutations_match_c() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let mut cases = vec![
            "info commands ::tcl::info::exists",
            "proc P {} {info exists [proc info {args} {return CUSTOM}]}; P",
            "interp alias {} IE {} info exists; proc P {} {set x 1; IE [format %s x]}; P",
            "interp alias {} I {} info; proc P {} {set x 1; I exists [format %s x]}; P",
            "set ::count 0; proc operand {} {incr ::count;return x}; proc P {} {set x 1;list [info exists [operand]] $::count}; P",
            "set ::calls 0; proc name {} {incr ::calls;return x}; proc P {} {set x 1; list [info exists [name]] $::calls}; P",
            "interp alias {} query_exists {} info exists; proc P {} {set x 1; query_exists [format %s x]}; P",
            "interp alias {} query_exists {} info exists; proc P {} {query_exists [proc info {args} {return CUSTOM}]}; P",
        ];
        if reference.version >= tcl_dialect::TclVersion::V8_5 {
            cases.extend([
                "list [namespace ensemble exists info] [info commands ::tcl::info::exists]",
                "set x 1; ::tcl::info::exists x",
                "proc replacement {args} {return CUSTOM}; namespace ensemble configure info -map [dict replace [namespace ensemble configure info -map] exists replacement]; proc P {} {info exists missing}; P",
                "rename ::tcl::info::exists ::old_exists; proc ::tcl::info::exists {args} {return CUSTOM}; proc P {} {info exists missing}; P",
                "proc P {} {set x 1; info exists [proc ::tcl::info::exists {args} {return CUSTOM}; format %s x]}; P",
                "proc helperA {args} {return A}; proc helperB {args} {return B}; namespace ensemble configure ::info -map {exists ::helperA}; proc P {} {info exists missing}; set first [P]; namespace ensemble configure ::info -map {exists ::helperB}; list $first [P]",
                "proc helperA {args} {return A}; proc helperB {args} {return B}; namespace ensemble configure ::info -map {exists ::helperA}; proc flip {} {namespace ensemble configure ::info -map {exists ::helperB};return missing}; proc Q {} {info exists [flip]}; list [Q] [Q]",
                "proc helperA {args} {return A}; proc helperB {args} {return B}; namespace ensemble configure ::info -map {exists ::helperA}; set ::count 0; proc flip {} {incr ::count; if {$::count > 1} {namespace ensemble configure ::info -map {exists ::helperB}};return missing}; proc P {} {info exists [flip]}; set first [P]; proc helperA args {}; list $first [P] $::count",
                "proc helperA {args} {return A}; proc helperB {args} {return B}; namespace ensemble configure ::info -map {exists ::helperA}; proc P {} {namespace ensemble configure ::info -map {exists ::helperB};info exists missing}; list [P] [P]",
            ]);
        }
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .expect("native info observation")
            .strict_text()
            .expect("native success");
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                result.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_yield_relay_preserves_lookup_namespace_and_resumer_frame() {
    let cases = [
        "namespace eval N {proc target args {list N [namespace current] [info level 0]};proc P {} {yieldto target ARG}};coroutine C N::P",
        "namespace eval N {proc P {} {yieldto namespace current}};coroutine C N::P",
        "namespace eval N {proc P {} {yieldto set x 1}};set first [coroutine C N::P];list $first [info exists ::N::x] [info exists ::x]",
        "namespace eval N {proc handler args {return CUSTOM};namespace unknown handler;proc P {} {yieldto missing ARG}};set status [catch {coroutine C N::P} message];list $status $message",
        "proc P {} {set value [yield FIRST];list $value [yieldto list A B]};list [coroutine C P] [C RESUMED] [C X Y]",
        "proc P {} {yieldto};set status [catch {coroutine C P} message];list $status $message",
    ];
    for reference in available_tclshs() {
        if reference.version < tcl_dialect::TclVersion::V8_6 {
            continue;
        }
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .expect("native coroutine observation")
            .strict_text()
            .expect("native success");
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                result.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

#[test]
fn actual_array_ensemble_private_commands_and_map_mutations_match_c() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let mut cases = vec![
            "info commands ::tcl::array::get",
            "array set a {k VALUE};list [array exists a] [array get a] [array size a]",
            "proc P {} {array set a {k VALUE};array get a};P",
        ];
        if reference.version >= tcl_dialect::TclVersion::V8_6 {
            cases.extend([
                "list [namespace ensemble exists array] [info commands ::tcl::array::get]",
                "array set a {k VALUE};::tcl::array::get a",
                "proc replacement args {return CUSTOM};namespace ensemble configure array -map {get ::replacement};array get a",
                "proc A args {return A};proc B args {return B};namespace ensemble configure array -map {get ::A};proc P {} {array get a};set first [P];namespace ensemble configure array -map {get ::B};list $first [P]",
                "proc A args {return A};proc B args {return B};namespace ensemble configure array -map {get ::A};proc flip {} {namespace ensemble configure array -map {get ::B};return a};proc P {} {array get [flip]};list [P] [P]",
                "array set a {k VALUE};proc P {} {array get [proc array args {return CUSTOM};format %s a]};P",
            ]);
        }
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .expect("native array observation")
            .strict_text()
            .expect("native success");
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                result.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

#[test]
fn output_lookup_after_write_traces_matches_selected_c_protocol() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let cases =
            tcl_test_support::variable_outputs::variable_output_lookup_scripts(reference.version);
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .expect("native output observation")
            .strict_text()
            .expect("native success");
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(&source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(
                result.code.is_ok(),
                "{} {source}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native,
                "{} {source}",
                profile.name
            );
        }
    }
}

fn compare(path: &Path, profile: &'static DialectProfile) {
    for (name, body, jim_body) in CASES {
        let body = if tcl_registry::InvocationDialect::of_profile(profile).family()
            == Some(tcl_dialect::model::Family::Jim)
        {
            jim_body.unwrap_or(body)
        } else {
            body
        };
        let source = format!("proc run {{}} {{{body}}}\nputs [run]\n");
        let native = run_script(path, source.as_bytes())
            .expect("native invocation")
            .strict_text()
            .expect("successful native observation");
        let mut vm = Vm::default();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let definitions = format!("proc run {{}} {{{body}}}");
        let defined = vm.eval_source(&definitions).expect("VM definition");
        assert!(
            defined.code.is_ok(),
            "{} {name}: {}",
            profile.name,
            defined.result.to_str()
        );
        let result = vm.eval_source("run").expect("VM invocation");
        assert!(
            result.code.is_ok(),
            "{} {name}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(
            result.result.to_str().as_ref(),
            native.as_str(),
            "{} {name}",
            profile.name
        );
    }
}

#[test]
fn native_compilation_timing_matches_actual_c_releases() {
    let references = available_tclshs();
    assert!(!references.is_empty(), "configure C Tcl references");
    for reference in references {
        compare(
            &reference.path,
            DialectProfile::find(reference.version.dialect_profile_name()).unwrap(),
        );
    }
}

#[test]
fn native_compilation_timing_matches_actual_current_jim() {
    let reference = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("validated Jim override")
    };
    if let Some(reference) = reference {
        compare(
            &reference.path,
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
    }
}

#[test]
fn jim_alias_retains_root_slot_and_caller_lookup_and_frame() {
    let reference = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("validated Jim override")
    };
    let Some(reference) = reference else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    for source in [
        "namespace eval N {alias a list}; list [info commands ::a] [info commands ::N::a]",
        "namespace eval A {proc tgt {} {return A}; alias ::A::a tgt}; namespace eval B {proc tgt {} {return B}; ::A::a}",
        "alias setx set x; proc P {} {setx value; set x}; P",
        "set code [catch {alias a a} result]; list $code $result",
    ] {
        let native = run_script(
            &reference.path,
            format!("puts [eval {{{source}}}]\n").as_bytes(),
        )
        .unwrap()
        .strict_text()
        .unwrap();
        let mut vm = Vm::default();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let result = vm.eval_source(source).unwrap();
        assert!(result.code.is_ok(), "{source}: {}", result.result.to_str());
        assert_eq!(result.result.to_str().as_ref(), native.as_str(), "{source}");
    }
}

#[test]
fn unresolved_foreign_native_preflight_is_a_host_admission_error() {
    use tcl_runtime_api::{
        NativeCompilationAdmissionError, NativeCompilationPreflight, NativeExecutionError,
    };
    let mut vm = Vm::default();
    let unresolved = tcl_bytecode::FunctionAsm {
        native_compilation_preflight: NativeCompilationPreflight::UnpresentedDefiniteFailure,
        ..Default::default()
    };
    assert!(matches!(
        vm.try_run_function(&unresolved),
        Err(NativeExecutionError::CompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired
        ))
    ));
    // An unresolved compiler is not a proved Tcl failure. The same host entry
    // gate must reject it before executing instructions or formal binding.
    let possible = tcl_bytecode::FunctionAsm {
        native_compilation_preflight: NativeCompilationPreflight::ProviderRequired,
        ..Default::default()
    };
    assert!(possible.native_compilation_failure.is_none());
    assert!(matches!(
        vm.try_run_function(&possible),
        Err(NativeExecutionError::CompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired
        ))
    ));
    let invalid = tcl_bytecode::FunctionAsm {
        native_compilation_failure: Some(tcl_runtime_api::NativeCompilationError {
            message: "missing presentation".into(),
            error_code: Some("NONE".into()),
            command_contexts: Vec::new(),
            body_line: 1,
        }),
        ..Default::default()
    };
    assert!(matches!(
        vm.try_run_function(&invalid),
        Err(NativeExecutionError::CompilationAdmission(
            NativeCompilationAdmissionError::InvalidErrorPresentation
        ))
    ));
}

#[test]
fn malformed_native_operation_ranges_are_host_admission_errors() {
    use tcl_bytecode::{FunctionAsm, Instruction, NativeOperationSelectionSite, Op};
    use tcl_runtime_api::{
        CommandBindingGuard, CommandBindingIdentity, NativeCompilationAdmissionError,
        NativeExecutionError,
    };
    let valid = NativeOperationSelectionSite {
        compiler_prerequisite: None,
        requirements: vec![CommandBindingIdentity::in_rooted_namespace(
            "::", "set", "set",
        )],
        guard: CommandBindingGuard::BeforeArguments,
        end: "done".into(),
        source: "set x".into(),
        span: tcl_lexer::Span::new(10, 15),
        namespace: tcl_runtime_api::ByteNamespacePath::root(),
        namespace_context: None,
    };
    let mut instruction = Instruction::new(Op::NOP, Vec::new());
    instruction.offset = 0;
    let mut body = FunctionAsm {
        instructions: vec![instruction],
        ..Default::default()
    };
    body.labels.insert("done".into(), 1);
    body.instructions[0]
        .native_operation_selections
        .push(valid.clone());
    assert!(body.validate_native_compilation_entry().is_ok());
    let mut malformed = Vec::new();
    let mut site = valid.clone();
    site.end = "missing".into();
    malformed.push(site);
    let mut site = valid.clone();
    site.requirements.clear();
    malformed.push(site);
    let mut site = valid.clone();
    site.source = tcl_lexer::SourceImage::default();
    malformed.push(site);
    let mut site = valid.clone();
    site.namespace = tcl_runtime_api::ByteNamespacePath::root();
    site.guard = CommandBindingGuard::ChunkEntry;
    site.requirements[0].guard = CommandBindingGuard::BeforeArguments;
    malformed.push(site);
    let mut site = valid;
    site.span = tcl_lexer::Span::empty(10);
    malformed.push(site);
    let mut vm = Vm::default();
    for site in malformed {
        body.instructions[0].native_operation_selections[0] = site;
        assert!(matches!(
            vm.try_run_function(&body),
            Err(NativeExecutionError::CompilationAdmission(
                NativeCompilationAdmissionError::NativePreflightRequired
            ))
        ));
    }
    body.labels.insert("done".into(), 0);
    assert!(body.validate_native_compilation_entry().is_err());
}

#[test]
fn native_compilation_failure_admission_matches_c_before_formals_and_stores() {
    let references = available_tclshs();
    assert!(!references.is_empty(), "configure C Tcl references");
    for reference in references {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for (parameters, body, invocation) in [
            ("x", "set", "P"),
            ("", "set ::early 1; set", "P"),
            ("x", "if {0} {set}; return OK", "P"),
        ] {
            let source = format!(
                "set ::early 0\nproc P {{{parameters}}} {{{body}}}\nset code [catch {{{invocation}}} message]\nlist $code $message $::early $::errorCode $::errorInfo"
            );
            let observation = format!("puts [eval {{{source}}}]\n");
            let native = run_script(&reference.path, observation.as_bytes())
                .unwrap()
                .strict_text()
                .unwrap();
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm.eval_source(&source).expect("native compiler admission");
            assert!(
                result.code.is_ok(),
                "{} {body}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                native.as_str(),
                "{} {body}",
                profile.name
            );
        }
    }
}

#[test]
fn native_compilation_trace_selection_matches_actual_c_releases() {
    let references = available_tclshs();
    assert!(!references.is_empty(), "configure C Tcl references");
    for reference in references {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for (command, literal, dynamic, watched) in [
            (
                "set",
                "set x [_install set 9]; format %s $x",
                "set target set; $target x [_install set 9]; format %s $x",
                "set x 9",
            ),
            (
                "return",
                "return [_install return 9]",
                "set target return; $target [_install return 9]",
                "return 9",
            ),
            (
                "list",
                "list [_install list 9]",
                "set target list; $target [_install list 9]",
                "list 9",
            ),
        ] {
            for (shape, body, before) in [
                ("during-argv-literal", literal, false),
                ("during-argv-dynamic", dynamic, false),
                ("before-entry", literal, true),
            ] {
                let definitions = format!(
                    "set ::hits {{}}\nset ::watched {{{watched}}}\n\
                     proc record {{args}} {{if {{[lindex $args 0] eq $::watched}} {{lappend ::hits HIT}}}}\n\
                     proc _install {{name value}} {{trace add execution $name enter record; format %s $value}}\n\
                     proc P {{}} {{{body}}}\n"
                );
                let observation = format!(
                    "{}set code [catch {{P}} result]; trace remove execution {command} enter record; format {{%s|%s|%s}} $code $result $::hits",
                    if before {
                        format!("trace add execution {command} enter record; ")
                    } else {
                        String::new()
                    }
                );
                // One outer evaluation avoids tclsh's stdin history recording
                // commands becoming additional trace observations.
                let source = format!("eval {{{definitions}puts [{observation}]}}\n");
                let native = run_script(&reference.path, source.as_bytes())
                    .expect("native invocation")
                    .strict_text()
                    .expect("native trace observation");
                let mut vm = Vm::default();
                vm.set_dialect_profile(profile);
                vm.set_compiler(Box::new(BytecodeCompileService::default()));
                let defined = vm.eval_source(&definitions).expect("VM definitions");
                assert!(
                    defined.code.is_ok(),
                    "{}: {}",
                    profile.name,
                    defined.result.to_str()
                );
                let result = vm.eval_source(&observation).expect("VM observation");
                assert!(
                    result.code.is_ok(),
                    "{} {command} {shape}: {}",
                    profile.name,
                    result.result.to_str()
                );
                assert_eq!(
                    result.result.to_str().as_ref(),
                    native.as_str(),
                    "{} {command} {shape}",
                    profile.name
                );
            }
        }
    }
}

#[test]
fn selected_native_set_keeps_live_variable_cell_traces() {
    let definitions = "set ::cell_hits 0\n\
        proc record_cell {args} {incr ::cell_hits}\n\
        proc install_cell_trace {value} {\
            uplevel 1 {trace add variable x write record_cell}; format %s $value\
        }\n\
        proc P {} {set x [install_cell_trace 9]; format {%s|%s} $x $::cell_hits}\n";
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let source = format!("eval {{{definitions}puts [P]}}\n");
        let native = run_script(&reference.path, source.as_bytes())
            .expect("native invocation")
            .strict_text()
            .expect("native variable trace observation");
        assert_eq!(native, "9|1", "{}", profile.name);
        let mut vm = Vm::default();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let defined = vm.eval_source(definitions).expect("VM definitions");
        assert!(defined.code.is_ok(), "{}", defined.result.to_str());
        let result = vm.eval_source("P").expect("VM observation");
        assert!(result.code.is_ok(), "{}", result.result.to_str());
        assert_eq!(
            result.result.to_str().as_ref(),
            native.as_str(),
            "{}",
            profile.name
        );
    }
}

#[test]
fn empty_lappend_updates_match_actual_native_read_and_write_traces() {
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for (initial, action, array) in [
            (Some(" A  B "), "", false),
            (Some("{"), "", false),
            (None, "", false),
            (Some("OLD"), "unset", false),
            (Some("OLD"), "recreate", false),
            (Some("OLD"), "unset", true),
            (Some("OLD"), "recreate", true),
        ] {
            let source = empty_lappend_source(
                initial,
                action,
                array,
                reference.version == tcl_dialect::TclVersion::V8_4,
            );
            let native = run_script(
                &reference.path,
                format!("puts [eval {{{source}}}]\n").as_bytes(),
            )
            .unwrap()
            .strict_text()
            .unwrap();
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let result = vm
                .try_eval_source(&source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            assert!(result.code.is_ok(), "{} {source}: {result:?}", profile.name);
            assert_eq!(
                result.result.to_str().as_ref(),
                native.trim_end(),
                "{} {source}",
                profile.name
            );
        }
    }
}

fn empty_lappend_source(
    initial: Option<&str>,
    action: &str,
    array: bool,
    old_trace: bool,
) -> String {
    let (base, name) = if array { ("a", "a(k)") } else { ("x", "x") };
    let init = initial.map_or_else(String::new, |value| {
        format!("set {name} {};", tcl_syntax::list::join_list([value]))
    });
    let action = match action {
        "unset" => format!("upvar 1 {base} v; unset v"),
        "recreate" => {
            let variable = if array { "v(k)" } else { "v" };
            format!("upvar 1 {base} v; unset v; set {variable} REPLACED")
        }
        _ => String::new(),
    };
    let trace = if old_trace {
        format!("trace variable {name} rw cb")
    } else {
        format!("trace add variable {name} {{read write}} cb")
    };
    format!(
        r#"set seen {{}}; proc cb {{n k op}} {{lappend ::seen $op; if {{$op eq "r" || $op eq "read"}} {{{action}}}}}; proc P {{}} {{{init} {trace}; set c [catch {{lappend {name}}} r]; list $c $r $::seen [info exists {name}]}}; P"#
    )
}

#[test]
fn native_append_selection_matches_shared_original_source_vectors() {
    use tcl_syntax::execution_conformance::{ExecutionDomain, vectors};
    let cases = vectors(ExecutionDomain::CommandBinding)
        .into_iter()
        .filter(|case| {
            case.id.starts_with("native_append_") || case.id.starts_with("native_lappend_")
        })
        .collect::<Vec<_>>();
    assert_eq!(cases.len(), 15, "retain every append selection control");
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        compare_append_vectors(&reference.path, profile, &cases);
    }
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("validated Jim override")
    };
    if let Some(jim) = jim {
        compare_append_vectors(
            &jim.path,
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            &cases,
        );
    }
}

fn compare_append_vectors(
    interpreter: &Path,
    profile: &'static DialectProfile,
    cases: &[tcl_syntax::execution_conformance::ExecutionVector],
) {
    for case in cases {
        let native = run_script(interpreter, case.script().as_bytes())
            .expect("actual append observation")
            .strict_text()
            .expect("native completion and result");
        let mut vm = Vm::default();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let completion = vm
            .try_eval_source(&case.source)
            .unwrap_or_else(|error| panic!("{} {}: {error:?}", profile.name, case.id));
        let observed = tcl_syntax::list::join_list([
            &completion.code.as_int().to_string(),
            completion.result.to_str().as_ref(),
        ]);
        assert_eq!(observed, native, "{} {}", profile.name, case.id);
    }
}

#[test]
fn tailcall_preserves_scheduled_activation_and_lookup_namespace_against_c() {
    let cases = [
        "proc target {} {return GLOBAL}; namespace eval N {proc target {} {return OWN}; proc P {} {tailcall target}}; N::P",
        "proc P {} {set x ISSUER; set t tailcall; $t set x AFTER}; proc caller {} {set x BEFORE; list [P] $x}; caller",
        "proc target {} {return TARGET}; proc P {} {set code [catch {tailcall target} result]; set ::seen [list $code $result]; return AFTER}; set result [P]; list $result $::seen",
        "proc target {} {return TARGET}; proc P {} {catch {tailcall target}; error FAIL}; P",
        "proc target {} {return TARGET}; proc P {} {catch {tailcall target}; catch {tailcall}; return AFTER}; P",
        "interp alias {} relay {} tailcall; proc P {} {relay set x AFTER}; proc caller {} {set x BEFORE; list [P] $x}; caller",
        "namespace eval N {proc target {} {yield FIRST; yield SECOND; return LAST}; proc P {} {tailcall target}}; set first [coroutine co N::P]; set second [co]; set last [co]; list $first $second $last [info commands co]",
        "proc target {} {return OLD}; proc leave {args} {rename target old; proc target {} {return NEW}}; proc P {} {tailcall target}; trace add execution P leave leave; P",
    ];
    for reference in available_tclshs()
        .into_iter()
        .filter(|reference| reference.version >= tcl_dialect::TclVersion::V8_6)
    {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in cases {
            let native = run_script(
                &reference.path,
                format!("set c [catch {{{source}}} r]; puts [list $c $r]\n").as_bytes(),
            )
            .expect("native scheduled tailcall observation")
            .strict_text()
            .expect("native completion and result");
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let completion = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{} {source}: {error:?}", profile.name));
            let observed = tcl_syntax::list::join_list([
                &completion.code.as_int().to_string(),
                completion.result.to_str().as_ref(),
            ]);
            assert_eq!(observed, native, "{} {source}", profile.name);
        }
    }
}

#[test]
fn namespace_code_retains_literal_builder_and_dynamic_private_worker() {
    let common = [
        "proc P {} {namespace code {my tick}}; P",
        "namespace eval N {proc P {} {namespace code {my tick}}}; N::P",
        "proc P {} {namespace code [list my tick]}; P",
        "proc P {} {namespace code {::namespace inscope :: my}}; P",
        "proc P {} {set result [namespace code {my tick}]; set result}; P",
        "proc P {} {catch {namespace code {my tick}} result; set result}; P",
        "proc namespace args {return CUSTOM}; proc P {} {namespace code {my tick}}; P",
        "proc change {} {rename namespace saved; proc namespace args {return CUSTOM}; return SCRIPT}; proc P {} {namespace code [change]}; P",
    ];
    let modern = [
        "proc ::tcl::namespace::code args {return CUSTOM}; proc P {} {namespace code {my tick}}; P",
        "proc change {} {rename ::tcl::namespace::code savedCode; proc ::tcl::namespace::code args {return CUSTOM}; return SCRIPT}; proc P {} {namespace code [change]}; P",
        "proc change {} {rename ::tcl::namespace::code savedCode; proc ::tcl::namespace::code args {return CUSTOM}; return SCRIPT}; proc P {} {catch {namespace code [change]} result; set result}; P",
        "proc P {} {try {error FAIL} on error {message options} {namespace code {my tick}}}; P",
    ];
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in common.into_iter().chain(
            modern
                .into_iter()
                .filter(|_| reference.version >= tcl_dialect::TclVersion::V8_6),
        ) {
            compare_namespace_usage(&reference.path, profile, source);
        }
    }
}

#[test]
fn namespace_origin_preserves_original_opcode_across_operand_mutation() {
    let common = [
        "proc target {} {return OK}; proc P {} {namespace origin target}; P",
        "namespace eval N {proc target {} {}; proc P {} {namespace origin target}}; N::P",
        "proc target {} {}; proc change {} {rename namespace saved; proc namespace args {return CUSTOM}; return target}; proc P {} {namespace origin [change]}; P",
        "proc target {} {}; proc namespace args {return CUSTOM}; proc P {} {namespace origin target}; P",
        "proc target {} {}; proc change {} {rename target replacement; return replacement}; proc P {} {namespace origin [change]}; P",
        "proc P {} {catch {namespace origin absent} result; list $result}; P",
    ];
    let modern = [
        "proc target {} {}; proc change {} {rename ::tcl::namespace::origin savedOrigin; proc ::tcl::namespace::origin args {return CUSTOM}; return target}; proc P {} {namespace origin [change]}; P",
        "proc target {} {}; proc ::tcl::namespace::origin args {return CUSTOM}; proc P {} {namespace origin target}; P",
        "proc target {} {}; proc change {} {rename ::tcl::namespace::origin savedOrigin; proc ::tcl::namespace::origin args {return CUSTOM}; return target}; proc P {} {catch {namespace origin [change]} result; set result}; P",
    ];
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in common.into_iter().chain(
            modern
                .into_iter()
                .filter(|_| reference.version >= tcl_dialect::TclVersion::V8_6),
        ) {
            compare_namespace_usage(&reference.path, profile, source);
        }
    }
    if let Some(jim) = locate_jimsh().expect("validated Jim override") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for source in common {
            compare_namespace_usage(&jim.path, profile, source);
        }
    }
}

#[test]
fn namespace_usage_and_alias_objv_dispatch_match_actual_native_contexts() {
    let common = [
        "namespace current EXTRA",
        "namespace cur EXTRA",
        "rename namespace NS; NS current EXTRA",
        "interp alias {} go {} namespace current; go EXTRA",
        "interp alias {} go {} return; proc P {} {go VALUE}; P",
        "interp alias {} go {} catch; go {error FAIL} result; list $result",
        "interp alias {} go {} set; proc P {} {set x OLD; go x NEW; set x}; P",
        "interp alias {} go {} upvar; proc P {} {go #0 ::x local; set local NEW}; P; set ::x",
        "interp alias {} go {} uplevel; proc P {} {set x OLD; go 1 {set x NEW}}; proc caller {} {set x BEFORE; P; set x}; caller",
        "proc target {v} {return OLD}; interp alias {} go {} target; go [proc target {v} {return NEW}]",
        "interp alias {} go {} set; set value {a\\
b}; go x $value; list $x",
    ];
    let modern = [
        "interp alias {} go {} target; proc target {} {yield WAIT; return DONE}; proc P {} {go}; set first [coroutine c P]; set second [c]; list $first $second",
        "::tcl::namespace::current EXTRA",
        "namespace ensemble create -command ens -map {cur ::tcl::namespace::current}; ens cur EXTRA",
        "namespace ensemble create -command ens -parameters {p} -map {cur ::tcl::namespace::current}; ens P cur EXTRA",
        "namespace ensemble create -command ens -map {cur {::tcl::namespace::current PREFIX}}; ens cur EXTRA",
        "namespace ensemble create -command inner -map {cur ::tcl::namespace::current}; namespace ensemble create -command outer -map {run inner}; outer run cur EXTRA",
    ];
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in common.into_iter().chain(
            modern
                .into_iter()
                .filter(|_| reference.version >= tcl_dialect::TclVersion::V8_6),
        ) {
            compare_namespace_usage(&reference.path, profile, source);
        }
    }
    if let Some(jim) = locate_jimsh().expect("validated Jim override") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for source in [
            "namespace current EXTRA",
            "alias go namespace current; go EXTRA",
        ] {
            compare_namespace_usage(&jim.path, profile, source);
        }
    }
}

fn compare_namespace_usage(path: &Path, profile: &'static DialectProfile, source: &str) {
    try_compare_namespace_usage(path, profile, source).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn binary_decoder_options_and_value_provenance_match_native_engines() {
    let sources = tcl_test_support::binary_values::BINARY_VALUE_SCRIPTS;
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for &source in sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    if let Some(reference) = locate_jimsh().expect("Jim Binary oracle") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for &source in sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn binary_strict_decoder_options_match_shared_native_observations() {
    let mut failures = Vec::new();
    for reference in available_tclshs()
        .into_iter()
        .filter(|reference| reference.version >= tcl_dialect::TclVersion::V8_6)
    {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for &source in tcl_test_support::binary_values::BINARY_DECODER_SCRIPTS {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
        for source in [
            "binary decode hex -strict {61 62}",
            "binary decode hex -str 61",
            "binary decode hex -strict -strict 61",
        ] {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn string_and_dict_live_worker_maps_match_native_compilation() {
    let mut failures = Vec::new();
    for reference in available_tclshs()
        .into_iter()
        .filter(|reference| reference.version >= tcl_dialect::TclVersion::V8_5)
    {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in [
            "string length abc",
            "dict get {k VALUE} k",
            "proc replacement args {return CUSTOM}; namespace ensemble configure string -map {length replacement}; string length abc",
            "proc replacement args {return CUSTOM}; namespace ensemble configure dict -map {get replacement}; dict get {k VALUE} k",
            "proc ::tcl::string::length args {return PRIVATE}; string length abc",
            "proc ::tcl::dict::get args {return PRIVATE}; dict get {k VALUE} k",
        ] {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn native_operation_replay_preserves_the_original_source_namespace() {
    let mut failures = Vec::new();
    for reference in available_tclshs()
        .into_iter()
        .filter(|reference| reference.version >= tcl_dialect::TclVersion::V8_5)
    {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in [
            "proc replacement args {return [list [namespace current] [info level]]}; namespace ensemble configure string -map {length ::replacement}; string length abc",
            "namespace eval N {proc replacement args {return [list [namespace current] [info level]]}; namespace ensemble configure ::string -map {length ::N::replacement}; string length abc}",
        ] {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jim_quoted_substitution_reconstructs_the_native_string_character_cache() {
    let Some(reference) = locate_jimsh().expect("Jim string cache oracle") else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let source = "set s [binary format H* c341c3a9];set r [string range $s 0 1];set q \"$r\";list [string length $r] [string length $q]";
    try_compare_namespace_usage(&reference.path, profile, source).unwrap();
}

#[test]
fn jim_string_comparison_and_case_follow_the_native_raw_unit_protocol() {
    let Some(reference) = locate_jimsh().expect("Jim comparison oracle") else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut failures = Vec::new();
    for &source in tcl_test_support::jim_strings::JIM_COMPARISON_AND_CASE_SCRIPTS {
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jim_string_search_and_repeat_follow_the_native_byte_protocol() {
    let Some(reference) = locate_jimsh().expect("Jim string search oracle") else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut failures = Vec::new();
    for &source in tcl_test_support::jim_strings::JIM_SEARCH_AND_REPEAT_SCRIPTS {
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jim_glob_matches_the_native_byte_and_numeric_unit_protocol() {
    let Some(reference) = locate_jimsh().expect("Jim glob oracle") else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut failures = Vec::new();
    for &source in tcl_test_support::jim_strings::JIM_GLOB_SCRIPTS {
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jim_trim_retains_native_physical_ownership_and_cached_count() {
    let Some(reference) = locate_jimsh().expect("Jim trim oracle") else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut failures = Vec::new();
    for &source in tcl_test_support::jim_strings::JIM_TRIM_SCRIPTS {
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jim_binary_uses_the_actual_scripted_compound_tailcall() {
    let Some(reference) = locate_jimsh().expect("Jim Binary compound oracle") else {
        return;
    };
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    for source in [
        "binary f a X",
        "binary encode hex X",
        "rename binary b; b format a X",
        "proc {binary format} args {return CUSTOM}; binary format a X",
        "proc {b format} args {return WRONG}; rename binary b; b format a X",
        "proc p {} {binary scan ABC a* result; list $result [info level]}; p",
    ] {
        compare_namespace_usage(&reference.path, profile, source);
    }
}

fn try_compare_namespace_usage(
    path: &Path,
    profile: &'static DialectProfile,
    source: &str,
) -> Result<(), String> {
    let native = run_script(
        path,
        format!("set c [catch {{{source}}} r]; puts [list $c $r]\n").as_bytes(),
    )
    .expect("actual invocation usage observation");
    assert!(
        native.success() && native.stderr.is_empty(),
        "native completion observation failed: {native:?}"
    );
    let native = native.stdout.strip_suffix(b"\n").unwrap_or(&native.stdout);
    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let completion = vm.try_eval_source(source).map_err(|error| {
        format!(
            "{} {source}: {error:?}; entry stage: {}",
            profile.name,
            diagnose_namespace_usage_entry(profile, source)
        )
    })?;
    let mut observed = Vec::new();
    tcl_syntax::list::append_list_element(
        &mut observed,
        completion.code.as_int().to_string().as_bytes(),
        true,
    );
    observed.push(b' ');
    tcl_syntax::list::append_list_element(&mut observed, &completion.result.string_bytes(), false);
    if observed != native {
        return Err(format!(
            "{} {source}: VM {observed:?}, native {native:?}",
            profile.name
        ));
    }
    Ok(())
}

// Failure-only localization uses natural outer command boundaries in a fresh
// interpreter. The original whole-source comparison remains authoritative.
fn diagnose_namespace_usage_entry(profile: &'static DialectProfile, source: &str) -> String {
    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    for command in
        tcl_compiler::segmenter::segment_commands_with_offset_and_config(source, 0, config)
    {
        let Some(text) = source.get(command.execution_span(source).as_range()) else {
            return "missing original command extent".to_owned();
        };
        match vm.try_eval_source(text) {
            Ok(completion) if completion.code.is_ok() => {}
            Ok(completion) => {
                return format!(
                    "{text:?} => {:?} {:?}",
                    completion.code,
                    completion.result.string_bytes()
                );
            }
            Err(error) => return format!("{text:?} => {error:?}"),
        }
    }
    "all natural entries admitted; whole-chunk compilation differs".to_owned()
}

#[test]
fn missing_alias_targets_preserve_native_handler_selection_and_resolution() {
    let common = [
        "proc unknown args {return ROOTPROC}; interp alias {} go {} absent; namespace eval N {proc unknown args {return LOCALPROC}; go}",
        "proc unknown args {return ROOTPROC}; namespace eval T {proc unknown args {return TARGETPROC}}; interp alias {} go {} T::absent; namespace eval N {proc unknown args {return LOCALPROC}; go}",
        "proc unknown args {return ROOTPROC}; rename unknown {}; interp alias {} go {} absent; go",
        "interp create child; child eval {proc unknown args {return CHILDPROC}}; interp alias {} go child absent; namespace eval N {proc unknown args {return LOCALPROC}; go}",
    ];
    let namespace_handlers = [
        "proc rootmiss args {return ROOTNS}; namespace unknown rootmiss; proc unknown args {return ROOTPROC}; interp alias {} go {} absent; namespace eval N {proc localmiss args {return LOCALNS}; namespace unknown localmiss; go}",
        "proc unknown args {return ROOTPROC}; namespace eval T {proc targetmiss args {return TARGETNS}; namespace unknown targetmiss}; interp alias {} go {} T::absent; go",
        "interp create child; child eval {proc miss args {return CHILDNS}; namespace unknown miss; proc unknown args {return CHILDPROC}}; interp alias {} go child absent; namespace eval N {proc localmiss args {return LOCALNS}; namespace unknown localmiss; go}",
        "interp create child; child eval {namespace eval T {proc miss args {return TARGETNS}; namespace unknown miss}; proc unknown args {return CHILDPROC}}; interp alias {} go child T::absent; go",
        "proc rootmiss args {return ROOTNS}; namespace unknown rootmiss; proc unknown args {return ROOTPROC}; interp alias {} go {} absent; namespace eval N {go}",
        "proc rootmiss args {return ROOTNS}; namespace unknown rootmiss; proc unknown args {return ROOTPROC}; interp alias {} go {} absent; namespace eval N {proc localmiss args {return LOCALNS}; namespace unknown ::N::localmiss; go}",
        "namespace eval N {proc localmiss args {return LOCALNS}; namespace unknown missingHandler; missingCommand}",
    ];
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in common.into_iter().chain(
            namespace_handlers
                .into_iter()
                .filter(|_| reference.version >= tcl_dialect::TclVersion::V8_5),
        ) {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    if let Some(reference) = locate_jimsh().expect("Jim fallback oracle") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for source in [
            "proc unknown args {return ROOTPROC}; namespace eval N {proc unknown args {return LOCALPROC}; alias go absent; go}",
            "alias go absent; go",
        ] {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn automatic_jim_errors_retain_actual_evaluation_frames_and_source_locations() {
    let Some(reference) = locate_jimsh().expect("Jim oracle discovery") else {
        return;
    };
    for (name, script) in tcl_test_support::automatic_errors::JIM_AUTOMATIC_ERROR_CASES {
        let native_source = format!("puts [eval {{{script}}}]\n");
        let expected = run_script(&reference.path, native_source.as_bytes())
            .expect("native Jim")
            .strict_text()
            .expect("native observation");
        let mut vm = Vm::default();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let result = vm
            .try_eval_source_at(
                script,
                tcl_runtime_api::script_source_location::ScriptSourceLocation {
                    file: "stdin".into(),
                    line: 1,
                },
            )
            .expect("VM native entry");
        assert!(result.code.is_ok(), "{name}: {}", result.result.to_str());
        assert_eq!(result.result.to_str().as_ref(), expected.as_str(), "{name}");
    }
}

#[test]
fn increment_retains_native_receiver_across_observer_lifetimes() {
    compare_receiver_lifetime_cases(
        |id| {
            id.starts_with("variable_rmw_")
                || id == "variable_unset_recreated_root_survives_old_members"
        },
        8,
    );
}

#[test]
fn array_unset_stages_preserve_original_cell_receivers() {
    compare_receiver_lifetime_cases(|id| id.starts_with("variable_array_unset_"), 8);
}

#[test]
fn list_index_native_selection_preserves_original_operands_and_dispatch_phase() {
    let mut sources = [
        "catch {lindex} result; set result",
        r"set malformed \{; lindex $malformed",
        "lindex {{A B} {C D}} {1 0}",
        "lindex {{A B} {C D}} 1 0",
        "set index 1; lindex {A B} $index",
        "proc P {} {lindex [proc lindex args {return CUSTOM}; list A B] 0}; P",
        "proc P {} {lindex {A B} [proc lindex args {return CUSTOM}; set index 0]}; P",
    ]
    .map(str::to_owned)
    .to_vec();
    sources.push(format!("lindex A{}", " 0".repeat(300)));
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in &sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    if let Some(reference) = locate_jimsh().expect("Jim list index oracle") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for source in &sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn binary_usage_retains_actual_public_private_and_rewritten_prefixes() {
    let common = ["binary format", "binary scan"];
    let modern = [
        "binary encode hex",
        "binary encode base64",
        "binary encode uuencode",
        "binary decode hex",
        "binary decode base64",
        "binary decode uuencode",
        "::tcl::binary::format",
        "::tcl::binary::scan",
        "::tcl::binary::encode::hex",
        "::tcl::binary::encode::base64",
        "::tcl::binary::decode::uuencode",
        "interp alias {} enc {} ::tcl::binary::encode::base64; enc",
        "interp alias {} enc {} binary encode base64; enc",
        "interp alias {} {codec name} {} ::tcl::binary::encode::hex; {codec name}",
        "namespace ensemble configure binary -map [dict create code ::tcl::binary::encode]; binary code base64",
    ];
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        let sources = common.iter().chain(
            modern
                .iter()
                .filter(|_| reference.version >= tcl_dialect::TclVersion::V8_6),
        );
        for source in sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    if let Some(reference) = locate_jimsh().expect("Jim binary usage oracle") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for source in common {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn array_unset_expression_callback_uses_actual_compilation_entry() {
    compare_receiver_lifetime_cases(
        |id| id == "variable_unset_recreated_root_survives_old_members",
        1,
    );
}

#[test]
fn array_unset_recreated_root_keeps_original_member_trace_receivers() {
    compare_receiver_lifetime_cases(
        |id| id == "variable_array_unset_captured_member_prefixes_ignore_recreated_root",
        1,
    );
}

fn compare_receiver_lifetime_cases(select_case: impl Fn(&str) -> bool, minimum: usize) {
    use tcl_syntax::execution_conformance::{ExecutionDomain, vectors};
    let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
        .into_iter()
        .filter(|case| select_case(&case.id))
        .collect();
    assert!(cases.len() >= minimum);
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for case in &cases {
            let source = if reference.version == tcl_dialect::TclVersion::V8_4 {
                case.source
                    .replace("trace add variable", "trace variable")
                    .replace("trace remove variable", "trace vdelete")
                    .replace(" read cb", " r cb")
            } else {
                case.source.clone()
            };
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, &source) {
                failures.push(format!("{}: {error}", case.id));
            }
        }
    }
    if let Some(reference) = locate_jimsh().expect("Jim increment oracle") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for case in cases
            .iter()
            .filter(|case| case.jim_want != tcl_syntax::execution_conformance::UNSUPPORTED)
        {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, &case.source)
            {
                failures.push(format!("{}: {error}", case.id));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn increment_amount_grammar_and_phase_match_actual_engines() {
    let sources = [
        "set x 10; incr x {1+2}",
        "set x 10; incr x {1/2}",
        "set x 10; incr x {int(2.5)}",
        "set x 10; incr x 3.0",
        "set x 10; incr x {[set ::seen 4]}",
        "set x 9223372036854775806; incr x 1",
        "set ::seen 0; set x 10; incr x [set ::seen [expr {$::seen+1}]]; list $x $::seen",
        "set x LEFT; incr x RIGHT",
    ];
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for source in sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    if let Some(reference) = locate_jimsh().expect("Jim increment amount oracle") {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for source in sources {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn increment_alias_preserves_a_previously_retired_namespace_owner() {
    let source = "namespace eval N {variable x 10}; proc p {} {upvar #0 N::x x; namespace delete N; catch {incr x} r; list $r}; p";
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn original_variable_operands_preserve_native_index_evaluation_and_lexical_axes() {
    use tcl_syntax::execution_conformance::{ExecutionDomain, vectors};
    let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
        .into_iter()
        .filter(|case| case.id.starts_with("variable_word_"))
        .collect();
    assert_eq!(cases.len(), 12, "retain every original variable control");
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for case in &cases {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, &case.source)
            {
                failures.push(error);
            }
        }
    }
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim variable-word oracle"))
    } else {
        locate_jimsh().expect("Jim variable-word oracle")
    };
    if let Some(reference) = jim {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for case in &cases {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, &case.source)
            {
                failures.push(error);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jim_expression_comparisons_and_conversion_errors_preserve_original_byte_objects() {
    let reference = require_jimsh().expect("required current Jim expression oracle");
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut failures = Vec::new();
    for &source in tcl_test_support::expressions::JIM_RAW_EXPRESSION_VALUE_SCRIPTS
        .iter()
        .chain(tcl_test_support::expressions::JIM_RAW_EXPRESSION_ERROR_SCRIPTS)
    {
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn expression_numeric_nul_inputs_follow_each_actual_native_scalar_getter() {
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        for &source in tcl_test_support::expressions::NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS {
            if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
                failures.push(error);
            }
        }
    }
    let reference = require_jimsh().expect("required current Jim numeric NUL oracle");
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    for &source in tcl_test_support::expressions::NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS {
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn expression_ordering_retains_actual_native_character_units() {
    let source = "set a 😀; set b \u{e000}; list [string length $a] [expr {$a < $b}] [expr {$a == $b}] [expr {$a eq $b}]";
    let mut failures = Vec::new();
    for reference in available_tclshs() {
        let profile = DialectProfile::find(reference.version.dialect_profile_name()).unwrap();
        if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
            failures.push(error);
        }
    }
    let reference = require_jimsh().expect("required current Jim character unit oracle");
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    if let Err(error) = try_compare_namespace_usage(&reference.path, profile, source) {
        failures.push(error);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
