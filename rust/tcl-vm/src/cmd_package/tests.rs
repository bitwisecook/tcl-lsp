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

use tcl_dialect::{TclVersion, version_satisfies as vsatisfies};
use tcl_runtime_api::Code;

use crate::interp::Vm;
use crate::value::Value;

fn vm() -> Vm {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::default(),
    ));
    vm
}

fn release(version: TclVersion) -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect(
        version.dialect_profile_name(),
    ))
}

fn jim() -> Vm {
    let mut vm = crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    crate::native_fixture::install_distribution(&mut vm);
    vm
}

fn package(vm: &mut Vm, words: &[&str]) -> tcl_runtime_api::Completion<Value> {
    let args: Vec<Value> = words.iter().map(|word| Value::string(*word)).collect();
    // `package require` defers its discovery scripts to the explicit
    // stack; a direct call settles them as the native invoke path does.
    let completion = super::cmd_package(vm, &args);
    vm.settle_native_invoke(completion)
}

fn selected_loader(
    vm: &mut Vm,
    name: &tcl_core_types::NameBytes,
    requirements: &[tcl_core_types::NameBytes],
    exact: bool,
    release: TclVersion,
) -> Option<super::SelectedLoader> {
    let operands = std::iter::once(name)
        .chain(requirements)
        .map(|operand| Value::new_native_string_bytes(operand.as_bytes()))
        .collect::<Vec<_>>();
    let mut selected = None;
    let completion = super::OriginalPackage::run(vm, false, &operands, |original| {
        selected = super::selected_loader(original, name, requirements, exact, release);
        crate::interp::ok(Value::empty())
    });
    assert_eq!(completion.code, Code::Ok);
    selected
}

#[test]
fn package_operands_preserve_native_storage_extents_and_error_bytes() {
    for release in TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(release);
        let resident = super::cmd_package(
            &mut vm,
            &[Value::from_native_string_bytes(&b"names\0junk"[..])],
        );
        assert_eq!(resident.code, Code::Ok, "{release:?}");
        let binary = super::cmd_package(&mut vm, &[Value::byte_array(&b"names\0junk"[..])]);
        assert_eq!(binary.code, Code::Error, "{release:?}");
        assert!(
            binary
                .result
                .string_bytes()
                .windows(2)
                .any(|bytes| bytes == b"\xc0\x80")
        );
        let raw = super::cmd_package(
            &mut vm,
            &[
                Value::string("vcompare"),
                Value::from_native_string_bytes(&b"\xff"[..]),
                Value::string("1"),
            ],
        );
        assert_eq!(
            raw.result.string_bytes().as_ref(),
            b"expected version number but got \"\xff\"",
            "{release:?}"
        );
        let counted = super::cmd_package(
            &mut vm,
            &[
                Value::string("vcompare"),
                Value::from_native_string_bytes(&b"1\0junk"[..]),
                Value::string("1"),
            ],
        );
        assert_eq!(counted.code, Code::Ok, "{release:?}");
        assert_eq!(counted.result.string_bytes().as_ref(), b"0");
    }
}

#[test]
fn actual_package_protocol_is_independent_of_logical_release() {
    for (native, logical, expected) in [
        (TclVersion::V8_4, TclVersion::V9_1, Code::Error),
        (TclVersion::V8_6, TclVersion::V9_1, Code::Error),
        (TclVersion::V9_0, TclVersion::V8_4, Code::Ok),
        (TclVersion::V9_1, TclVersion::V8_6, Code::Ok),
    ] {
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(
            tcl_dialect::DialectProfile::find(native.dialect_profile_name()).unwrap(),
        ));
        vm.set_runtime_version(logical);
        let result = super::cmd_package(
            &mut vm,
            &[
                Value::string("vcompare"),
                Value::from_native_string_bytes(b"1+\xff".as_slice()),
                Value::string("1"),
            ],
        );
        assert_eq!(
            result.code, expected,
            "native={native:?}, logical={logical:?}"
        );
        if expected == Code::Ok {
            assert_eq!(result.result.string_bytes().as_ref(), b"0");
        } else {
            assert_eq!(
                result.result.string_bytes().as_ref(),
                b"expected version number but got \"1+\xff\""
            );
        }
    }
}

#[test]
fn package_loader_entries_keep_first_spelling_and_registration_order() {
    for release in TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(release);
        assert_eq!(
            package(&mut vm, &["ifneeded", "p", "1.0", "OLD"]).code,
            Code::Ok
        );
        assert_eq!(
            package(&mut vm, &["ifneeded", "p", "1", "NEW"]).code,
            Code::Ok
        );
        assert_eq!(
            package(&mut vm, &["ifneeded", "p", "1.00"])
                .result
                .string_bytes()
                .as_ref(),
            b"NEW"
        );
        assert_eq!(
            package(&mut vm, &["ifneeded", "p", "2", "SECOND"]).code,
            Code::Ok
        );
        assert_eq!(
            package(&mut vm, &["versions", "p"])
                .result
                .string_bytes()
                .as_ref(),
            b"1.0 2"
        );
        assert_eq!(package(&mut vm, &["provide", "q", "1"]).code, Code::Ok);
        assert_eq!(
            package(&mut vm, &["versions", "q"])
                .result
                .string_bytes()
                .as_ref(),
            b""
        );
        let registered = super::cmd_package(
            &mut vm,
            &[
                Value::string("ifneeded"),
                Value::string("script"),
                Value::string("1"),
                Value::from_native_string_bytes(&b"PREFIX\0SUFFIX"[..]),
            ],
        );
        assert_eq!(registered.code, Code::Ok);
        assert_eq!(
            package(&mut vm, &["ifneeded", "script", "1"])
                .result
                .string_bytes()
                .as_ref(),
            b"PREFIX"
        );
    }
}

#[test]
fn package_tcl9_loader_suffixes_retain_opaque_bytes() {
    for release in [TclVersion::V9_0, TclVersion::V9_1] {
        let mut vm = Vm::new();
        vm.set_runtime_version(release);
        let registered = super::cmd_package(
            &mut vm,
            &[
                Value::string("ifneeded"),
                Value::string("raw"),
                Value::from_native_string_bytes(&b"1+\xff"[..]),
                Value::string("RAW"),
            ],
        );
        assert_eq!(registered.code, Code::Ok);
        assert_eq!(
            package(&mut vm, &["ifneeded", "raw", "1.0+different", "REPLACED"]).code,
            Code::Ok
        );
        assert_eq!(
            package(&mut vm, &["versions", "raw"])
                .result
                .string_bytes()
                .as_ref(),
            b"1+\xff"
        );
        assert_eq!(
            package(&mut vm, &["ifneeded", "raw", "1+query"])
                .result
                .string_bytes()
                .as_ref(),
            b"REPLACED"
        );
    }
}

#[test]
fn import_redefinition_rename_reexport_and_retirement_follow_dialect() {
    let jim = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let script = r"namespace eval S {proc p {} {return ONE}; namespace export *}; namespace eval D {namespace import ::S::*; namespace export *}; namespace eval E {namespace import ::D::*}; namespace eval S {proc p {} {return TWO}}; set result [list [D::p] [E::p] [namespace origin E::p]]; rename S::p S::moved; lappend result [catch {D::p} value] [catch {namespace origin E::p} value]; proc S::p {} {return THREE}; lappend result [D::p] [E::p]; rename S::moved {}; lappend result [llength [info commands D::p]] [llength [info commands E::p]]; set result";
    for profile in TclVersion::ALL
        .into_iter()
        .map(|release| {
            tcl_dialect::DialectProfile::find(release.dialect_profile_name()).expect("C profile")
        })
        .chain(std::iter::once(jim))
    {
        let mut vm = crate::native_fixture::interpreter(profile);
        if std::ptr::eq(profile, jim) {
            crate::native_fixture::install_distribution(&mut vm);
        }
        let result = vm.eval_source(script).expect("source");
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        let expected = if profile.namespace_import_binding()
            == Some(tcl_dialect::NamespaceImportBinding::SourceName)
        {
            "TWO TWO ::S::p 1 1 THREE THREE 1 1"
        } else {
            "TWO TWO ::S::p 0 0 TWO TWO 0 0"
        };
        assert_eq!(&*result.result.to_str(), expected, "{}", profile.name);
    }
}

#[test]
fn jim_import_ignores_exports_overwrites_and_retains_missing_source_alias() {
    let mut vm = jim();
    let result = vm.eval_source(r"namespace eval S {proc p {} {return SOURCE}}; namespace eval D {proc p {} {return DESTINATION}; namespace import -force ::S::*; set imported [namespace import]}; rename S::p {}; set result [list [llength [info commands D::p]] [catch {D::p}] [catch {namespace origin D::p}] [namespace eval D {namespace import ::missing::*}] [namespace eval D {namespace export}]]; proc S::p {} {return REVIVED}; lappend result [D::p] [namespace origin D::p] [catch {namespace path}]; set result").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "1 1 1 {} {} REVIVED ::S::p 1");
}

#[test]
fn jim_interpreter_handle_factory_alias_frame_and_deletion() {
    let mut vm = jim();
    let result = vm.eval_source(r"set ::x GLOBAL; namespace eval ::N {proc observe {} {set x LOCAL; set child [interp]; $child alias readX set x; $child alias where namespace current; set result [list [$child eval {readX}] [$child eval {where}]]; $child delete; return $result}}; ::N::observe").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "LOCAL ::N");
}

#[test]
fn jim_interpreter_handle_isolation() {
    let mut vm = jim();
    let result = vm.eval_source(r"namespace eval ::N {proc probe {} {return PARENT}}; package provide Isolated 2.0; set child [interp]; $child eval {namespace eval ::N {proc probe {} {return CHILD}}}; set result [list [::N::probe] [$child eval {::N::probe}] [$child eval {catch {package require Isolated}}]]; $child delete; set result").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "PARENT CHILD 1");
}

#[test]
fn legacy_upvar_level_presence_rejects_numeric_variable_pair() {
    for (version, expected) in [
        (TclVersion::V8_4, "1"),
        (TclVersion::V8_5, "1"),
        (TclVersion::V8_6, "0"),
        (TclVersion::V9_0, "0"),
        (TclVersion::V9_1, "0"),
    ] {
        let mut vm = release(version);
        let result = vm
            .eval_source(
                "proc probe {} {upvar 1 local}; proc caller {} {set 1 3; catch probe}; caller",
            )
            .expect("source");
        assert_eq!(result.code, Code::Ok);
        assert_eq!(&*result.result.to_str(), expected, "{version:?}");
    }
}

#[test]
fn upvar_qualified_scalar_and_element_use_selected_frame() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let (mut vm, expected) = if dialect == "jim" {
            (jim(), "11 12 4 5")
        } else {
            (
                crate::native_fixture::interpreter(
                    tcl_dialect::DialectProfile::find(dialect).expect("profile"),
                ),
                "11 12 11 12",
            )
        };
        let result = vm.eval_source(r"namespace eval N {namespace eval R {variable x 4; variable a; set a(k) 5}; proc outer {} {inner::scalar; inner::array; list $R::x $R::a(k) $::N::R::x $::N::R::a(k)}; namespace eval inner {namespace eval R {variable x 99; variable a; set a(k) 98}; proc scalar {} {upvar 1 R::x alias; set alias 11}; proc array {} {upvar 1 R::a(k) alias; set alias 12}}}; N::outer").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), expected, "{dialect}");
        let result = vm
            .eval_source(
                "proc global_alias {} {upvar #0 ::N::R::a(k) alias; incr alias}; global_alias",
            )
            .expect("source");
        assert_eq!(result.code, Code::Ok);
        assert_eq!(
            &*result.result.to_str(),
            if dialect == "jim" { "6" } else { "13" }
        );
    }
}

#[test]
fn qualified_variable_fallback_depends_on_cell_and_release() {
    for (version, expected) in [
        (TclVersion::V8_4, "11 0"),
        (TclVersion::V8_6, "11 0"),
        (TclVersion::V9_0, "9 1"),
    ] {
        let mut vm = release(version);
        let result = vm.eval_source("namespace eval R {variable x 9}; namespace eval N {namespace eval R {}; proc p {} {set R::x 11}}; N::p; list $::R::x [info exists ::N::R::x]").expect("source");
        assert_eq!(result.code, Code::Ok);
        assert_eq!(&*result.result.to_str(), expected, "{version:?}");
    }
}

#[test]
fn jim_namespace_delete_retires_flat_variable_cells() {
    let mut vm = jim();
    let result = vm.eval_source("namespace eval N {variable v 4; namespace eval C {variable x 3}}; namespace delete N; list [info exists ::N::v] [info exists ::N::C::x]").expect("source");
    assert_eq!(result.code, Code::Ok);
    assert_eq!(&*result.result.to_str(), "0 0");
}

#[test]
fn jim_activation_variables() {
    let mut vm = jim();
    let result = vm.eval_source(r"namespace eval N {set ephemeral 5; variable v 7; list $ephemeral $v [info locals] [info vars]}; list [info exists ::N::ephemeral] [set ::N::v]").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "0 7");
}

#[test]
fn jim_relative_qualified_proc_variable() {
    let mut vm = jim();
    let result = vm.eval_source(r"namespace eval N {variable R::x 9; proc p {} {set R::x 10; list $R::x $::N::R::x}}; N::p").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "10 9");
}

#[test]
fn jim_flat_absolute_variable_names() {
    let mut vm = jim();
    let result = vm.eval_source(r"set ::Missing::x 3; set ::N:::x 4; set ::N::x 5; list $::::Missing::x $::N:::x $::N::x").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "3 4 5");
}

#[test]
fn jim_namespace_root_is_local_activation() {
    let mut vm = jim();
    let result = vm
        .eval_source(r"namespace eval :: {set ephemeralRoot 11}; info exists ::ephemeralRoot")
        .expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "0");
}

#[test]
fn jim_upvar_targets_selected_activation() {
    let mut vm = jim();
    let result = vm.eval_source(r"proc inner {} {upvar 1 R::x link; set link 12}; proc outer {} {set R::x 3; inner; set R::x}; outer").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "12");
}

#[test]
fn jim_variable_declaration_preserves_colon_runs() {
    let mut vm = jim();
    let result = vm.eval_source(r"namespace eval N {variable R:::v 6; incr v}; list [set ::N::R:::v] [info exists ::N::R::v]").expect("source");
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
    assert_eq!(&*result.result.to_str(), "7 0");
}

#[test]
fn jim_direct_packages_use_global_source_and_preserve_partial_effects() {
    let directory =
        std::env::temp_dir().join(format!("2286-jim-package-vm-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("package directory");
    for (name, source) in [
        (
            "plain",
            "set ::loaderContext [list [namespace current] [info level]]; return done",
        ),
        (
            "recursive",
            "set ::recursiveVersion [package require recursive 999]; package provide recursive invalid",
        ),
        (
            "bad",
            "proc ::retainedSideEffect {} {return yes}; package provide bad 999; error stop",
        ),
    ] {
        std::fs::write(directory.join(format!("{name}.tcl")), source).expect("package source");
    }
    let mut vm = jim();
    let setup = format!(
        "set ::auto_path [list {}]",
        tcl_syntax::list::list_element(&directory.to_string_lossy())
    );
    assert!(vm.eval_source(&setup).expect("setup").code.is_ok());
    assert_eq!(
        &*vm.eval_source("namespace eval Probe {proc p {} {package require plain 999}; p}")
            .expect("require")
            .result
            .to_str(),
        "1.0"
    );
    assert_eq!(
        &*vm.eval_source("set ::loaderContext")
            .expect("context")
            .result
            .to_str(),
        ":: 0"
    );
    assert_eq!(
        &*package(&mut vm, &["require", "recursive"]).result.to_str(),
        "1.0"
    );
    assert_eq!(
        &*vm.eval_source("set ::recursiveVersion")
            .expect("recursive")
            .result
            .to_str(),
        ""
    );
    assert!(!package(&mut vm, &["require", "bad"]).code.is_ok());
    assert_eq!(vm.package_version("bad"), None);
    assert_eq!(
        &*vm.eval_source("retainedSideEffect")
            .expect("retained")
            .result
            .to_str(),
        "yes"
    );
    assert!(!package(&mut vm, &["provide", "plain"]).code.is_ok());
    std::fs::remove_dir_all(directory).expect("remove fixtures");
}

#[test]
fn legacy_package_arity_and_unknown_callback_follow_core_protocol() {
    let mut vm = release(TclVersion::V8_4);
    assert!(!package(&mut vm, &["require", "p", "1", "2"]).code.is_ok());
    assert!(
        !package(&mut vm, &["vsatisfies", "1", "1", "2"])
            .code
            .is_ok()
    );
    assert!(!package(&mut vm, &["provide", "p", "1.0a1"]).code.is_ok());
    assert!(
        vm.eval_source("proc probe {args} {set ::observed $args}; package unknown probe")
            .expect("valid callback")
            .code
            .is_ok()
    );
    assert!(
        !package(&mut vm, &["require", "-exact", "absent", "1.0"])
            .code
            .is_ok()
    );
    assert_eq!(
        vm.eval_source("set ::observed")
            .expect("callback observation")
            .result
            .to_str()
            .as_ref(),
        "absent 1.0 -exact"
    );
}

/// The pre-provided core packages follow the pinned release, so
/// `package require Tcl 8.5` fails under a 9.x pin exactly as `tclsh9.0`
/// fails it (`version conflict for package "Tcl": have 9.0.4, need 8.5`).
/// Hardcoding `9.0.4` and providing `Tcl`+`tcl` regardless of the pin
/// would make that require wrongly *succeed*.
///
/// Measured (`package provide <name>` in a fresh `tclsh`): 8.4.20 →
/// `Tcl` = `8.4` and no `tcl`; 8.5.19 → `Tcl` = `8.5.19`; 8.6.14 →
/// `Tcl` = `8.6.14` with `TclOO` = `1.1.0`; 9.0.4 and 9.1b0 → all four
/// names, at the patch level and `1.3.1`.
#[test]
fn core_provides_follow_the_pinned_release() {
    for version in TclVersion::ALL {
        let vm = release(version);
        for core in version.core_provided_packages() {
            assert_eq!(
                vm.package_version(core.name),
                Some(core.version),
                "{version:?} provides {}",
                core.name
            );
        }
        // FN guard: a name this release does not pre-provide is absent,
        // not left over from the construction-time default pin (9.0).
        if version < TclVersion::V9_0 {
            assert_eq!(vm.package_version("tcl"), None, "{version:?}");
            assert_eq!(vm.package_version("tcl::oo"), None, "{version:?}");
        }
        if version < TclVersion::V8_6 {
            assert_eq!(vm.package_version("TclOO"), None, "{version:?}");
        }
    }
}

/// `package require Tcl 8.5` means `[8.5, 9)`: satisfied on 8.5 and 8.6,
/// a version conflict on 8.4 and on every 9.x — the tclsh answers
/// measured on all five reference interpreters.
#[test]
fn package_require_tcl_85_matches_tclsh_per_release() {
    for version in TclVersion::ALL {
        let vm = release(version);
        let provided = vm
            .package_version("Tcl")
            .expect("Tcl is provided")
            .to_owned();
        let satisfied = vsatisfies(&provided, "8.5");
        assert_eq!(
            satisfied,
            matches!(version, TclVersion::V8_5 | TclVersion::V8_6),
            "{version:?}: `package require Tcl 8.5` against provided {provided}"
        );
    }
}

#[test]
fn version_ranges() {
    assert!(vsatisfies("9.0", "8.5-"));
    assert!(vsatisfies("9.0", "9.0-"));
    assert!(!vsatisfies("8.4", "8.5-"));
    assert!(vsatisfies("8.6", "8.5"));
    assert!(!vsatisfies("9.0", "8.5")); // 8.5 → [8.5, 9)
    assert!(vsatisfies("8.5.2", "8.5-9.0"));
    assert!(!vsatisfies("9.0", "8.5-9.0"));
}

#[test]
fn malformed_requirements_precede_an_active_loader_cycle() {
    let mut vm = vm();
    vm.begin_package_loading("p", "1.0");
    let malformed = package(&mut vm, &["require", "p", "invalid"]);
    assert_eq!(malformed.code, Code::Error);
    assert_eq!(
        &*malformed.result.to_str(),
        "expected version number but got \"invalid\""
    );

    let cycle = package(&mut vm, &["require", "p", "1.0"]);
    assert_eq!(cycle.code, Code::Error);
    assert_eq!(
        &*cycle.result.to_str(),
        "circular package dependency: attempt to provide p 1.0 requires p 1.0"
    );
    vm.end_package_loading("p", "1.0");
}

#[test]
fn package_commands_apply_the_pinned_plus_suffix_policy() {
    let mut tcl8 = release(TclVersion::V8_6);
    let rejected = package(&mut tcl8, &["provide", "p", "1.2+platform"]);
    assert_eq!(rejected.code, Code::Error);
    assert_eq!(
        &*rejected.result.to_str(),
        "expected version number but got \"1.2+platform\""
    );
    assert_eq!(tcl8.package_version("p"), None);
    let rejected = package(&mut tcl8, &["ifneeded", "p", "1.2+platform", ""]);
    assert_eq!(rejected.code, Code::Error);
    assert_eq!(
        tcl8.package_ifneeded_versions("p"),
        [] as [std::string::String; 0]
    );

    let mut tcl9 = release(TclVersion::V9_0);
    let provided = package(&mut tcl9, &["provide", "p", "1.2+platform"]);
    assert_eq!(provided.code, Code::Ok);
    assert_eq!(tcl9.package_version("p"), Some("1.2+platform"));
    let compared = package(&mut tcl9, &["vcompare", "1.2+platform", "1.2"]);
    assert_eq!(compared.code, Code::Ok);
    assert_eq!(&*compared.result.to_str(), "0");
    let registered = package(&mut tcl9, &["ifneeded", "q", "1.2+platform", ""]);
    assert_eq!(registered.code, Code::Ok);
    assert_eq!(tcl9.package_ifneeded_versions("q"), vec!["1.2+platform"]);

    let mut exact_provider = release(TclVersion::V9_0);
    assert_eq!(
        package(&mut exact_provider, &["ifneeded", "q", "1.2+platform", ""]).code,
        Code::Ok
    );
    assert_eq!(
        package(&mut exact_provider, &["ifneeded", "q", "1.3", ""]).code,
        Code::Ok
    );
    let exact_loader = selected_loader(
        &mut exact_provider,
        &tcl_core_types::NameBytes::from("q"),
        &[tcl_core_types::NameBytes::from("1.2+platform")],
        true,
        TclVersion::V9_0,
    )
    .expect("exact Tcl 9 provider selection");
    assert_eq!(exact_loader.version, "1.2+platform");

    let mut exact = release(TclVersion::V9_0);
    assert_eq!(package(&mut exact, &["provide", "p", "1.3"]).code, Code::Ok);
    let mismatch = package(&mut exact, &["require", "-exact", "p", "1.2+x"]);
    assert_eq!(mismatch.code, Code::Error);
    assert_eq!(
        &*mismatch.result.to_str(),
        "version conflict for package \"p\": have 1.3, need exactly 1.2+x"
    );
    let match_result = package(&mut exact, &["require", "-exact", "p", "1.3+x"]);
    assert_eq!(match_result.code, Code::Ok);
    assert_eq!(&*match_result.result.to_str(), "1.3");

    // A profile change does not rewrite package state. Selection for an
    // 8.6 release does not pick a Tcl 9-only provider, while the command
    // itself keeps the actual 9.0 engine's protocol, as tclsh9.0 answers.
    tcl9.set_runtime_version(TclVersion::V8_6);
    assert!(
        selected_loader(
            &mut tcl9,
            &tcl_core_types::NameBytes::from("q"),
            &[],
            false,
            TclVersion::V8_6
        )
        .is_none()
    );
    let required = package(&mut tcl9, &["require", "p", "1.2"]);
    assert_eq!(required.code, Code::Ok);
    assert_eq!(&*required.result.to_str(), "1.2+platform");
    assert_eq!(tcl9.package_version("p"), Some("1.2+platform"));
}
#[test]
fn provided_version_headers_match_all_native_30_windows() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    use tcl_syntax::scalar_getter::NativeScalarGetterKind;
    const FIXTURES: [&str; 5] = [
        include_str!("../../../tcl-registry/tests/data/native_package_versions/8.4.20.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_versions/8.5.19.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_versions/8.6.18.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_versions/9.0.4.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_versions/9.1.0.tsv"),
    ];
    fn class(value: &Value) -> &'static str {
        match value.native_object_snapshot().cache {
            Cache::None => "none",
            Cache::Numeric(_) => "int",
            other => panic!("unexpected version primary {other:?}"),
        }
    }
    let mut compared = 0;
    for (version, fixture) in TclVersion::ALL.into_iter().zip(FIXTURES) {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        assert_eq!(package(&mut vm, &["provide", "probe", "12"]).code, Code::Ok);
        let mut latest = package(&mut vm, &["provide", "probe"]);
        let held = latest.result.clone();
        let rows: Vec<Vec<&str>> = fixture
            .lines()
            .map(|row| tcl_test_support::fixture::tsv_fields(row).collect())
            .collect();
        assert_eq!(class(&held), rows[0][2]);
        assert_eq!(held.native_object_reference_count().to_string(), rows[0][3]);
        compared += 1;
        held.native_scalar_probe(vm.native_invocation_dialect(), NativeScalarGetterKind::Int)
            .unwrap()
            .unwrap();
        assert_eq!(class(&held), rows[1][3]);
        assert_eq!(rows[1][2], "12");
        compared += 1;
        for (member, row) in ["provide", "require", "present"]
            .into_iter()
            .zip(&rows[2..5])
        {
            latest = package(&mut vm, &[member, "probe"]);
            assert_eq!(latest.code, Code::Ok);
            assert_eq!(row[1], "0");
            assert_eq!(
                usize::from(
                    latest.result.native_object_identity() == held.native_object_identity()
                )
                .to_string(),
                row[2]
            );
            assert_eq!(class(&latest.result), row[3]);
            assert_eq!(held.native_object_reference_count().to_string(), row[4]);
            compared += 1;
        }
        latest = package(&mut vm, &["forget", "probe"]);
        assert_eq!(latest.code, Code::Ok);
        assert_eq!(held.native_object_reference_count().to_string(), rows[5][2]);
        assert_eq!(class(&held), rows[5][3]);
        compared += 1;
    }
    assert_eq!(compared, 30);
}

#[test]
fn package_files_headers_match_all_native_10_windows_and_scope_lifetimes() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    const FIXTURES: [&str; 2] = [
        include_str!("../../../tcl-registry/tests/data/native_package_files/9.0.4.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_files/9.1.0.tsv"),
    ];
    let mut compared = 0;
    for (version, fixture) in [TclVersion::V9_0, TclVersion::V9_1]
        .into_iter()
        .zip(FIXTURES)
    {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        vm.begin_package_loading(b"probe", b"1");
        vm.record_package_source_file(b"leaf.tcl").unwrap();
        vm.end_package_loading(b"probe", b"1");
        let rows: Vec<Vec<&str>> = fixture
            .lines()
            .map(|row| tcl_test_support::fixture::tsv_fields(row).collect())
            .collect();
        vm.with_package_file_object(b"probe", |original| {
            assert!(matches!(
                original.native_object_snapshot().cache,
                Cache::List { .. }
            ));
            assert_eq!(
                original.native_object_reference_count().to_string(),
                rows[0][2]
            );
        });
        assert_eq!(rows[0][3], "list");
        compared += 1;
        let mut latest = package(&mut vm, &["files", "probe"]);
        assert_eq!(latest.code, Code::Ok);
        assert_eq!(rows[1][1], "0");
        assert_eq!(
            latest.result.native_object_reference_count().to_string(),
            rows[1][3]
        );
        compared += 1;
        let held = latest.result.clone();
        assert_eq!(held.native_object_reference_count().to_string(), rows[2][3]);
        compared += 1;
        latest = package(&mut vm, &["forget", "probe"]);
        assert_eq!(latest.code, Code::Ok);
        assert_eq!(held.native_object_reference_count().to_string(), rows[3][3]);
        compared += 1;
        latest = crate::interp::ok(held.clone());
        assert_eq!(
            latest.result.native_object_identity(),
            held.native_object_identity()
        );
        assert_eq!(held.native_object_reference_count().to_string(), rows[4][3]);
        compared += 1;
        drop(latest);
        drop(held);
        vm.begin_package_loading(b"outer", b"1");
        vm.record_package_source_file(b"first.tcl").unwrap();
        let detached = vm.take_package_file_scope();
        vm.record_package_source_file(b"excluded.tcl").unwrap();
        vm.begin_package_loading(b"nested", b"1");
        vm.record_package_source_file(b"nested.tcl").unwrap();
        vm.end_package_loading(b"nested", b"1");
        vm.restore_package_file_scope(detached);
        vm.forget_package_completely(b"outer");
        vm.record_package_source_file(b"after-forget.tcl").unwrap();
        vm.end_package_loading(b"outer", b"1");
        assert_eq!(
            vm.take_package_file_scope(),
            [] as [tcl_core_types::NameBytes; 0]
        );
        assert_eq!(
            vm.package_file_object(b"outer").string_bytes().as_ref(),
            b"after-forget.tcl"
        );
        assert_eq!(
            vm.package_file_object(b"nested").string_bytes().as_ref(),
            b"nested.tcl"
        );
    }
    assert_eq!(compared, 10);
}

#[test]
fn shared_package_files_append_is_an_outer_native_fatal_condition() {
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        vm.begin_package_loading(b"probe", b"1");
        vm.record_package_source_file(b"leaf.tcl").unwrap();
        let retained = vm.package_file_object(b"probe");
        let error = vm.record_package_source_file(b"second.tcl").unwrap_err();
        assert!(matches!(
            error,
            tcl_syntax::value::ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::SharedPackageFileListMutation
            )
        ));
        assert_eq!(retained.string_bytes().as_ref(), b"leaf.tcl");
    }
}

#[test]
fn package_error_publication_matches_all_21_original_native_controls() {
    // Native proof: naming.package.error-publication-bad-member
    // docs/design/analysis/name-resolution-proofs/package-error-publication-bad-member.md
    // Native proof: naming.package.error-publication-bad-preference
    // docs/design/analysis/name-resolution-proofs/package-error-publication-bad-preference.md
    // Native proof: naming.package.error-publication-unknown-completion
    // docs/design/analysis/name-resolution-proofs/package-error-publication-unknown-completion.md
    // Native proof: naming.package.error-publication-loader-completion
    // docs/design/analysis/name-resolution-proofs/package-error-publication-loader-completion.md
    fn decode(text: &str) -> Vec<u8> {
        assert!(text.len().is_multiple_of(2));
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let fixture = include_str!(
        "../../../tcl-registry/tests/data/native_package_error_publication/native.tsv"
    );
    let mut compared = 0;
    for row in fixture.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        assert_eq!(fields.len(), 4);
        let profile = crate::environment::profile_for_dialect(fields[0]);
        let mut vm = crate::native_fixture::interpreter(profile);
        let script = decode(fields[2]);
        let completion = vm
            .eval_source(std::str::from_utf8(&script).unwrap())
            .expect("original package failure source");
        assert!(
            vm.refused_completion().is_none(),
            "{} {}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "{} {}",
            fields[0],
            fields[1]
        );
        let strings = vm.actual_native_string_protocol().unwrap();
        assert_eq!(
            completion
                .result
                .native_string_bytes(strings)
                .unwrap()
                .as_ref(),
            decode(fields[3]).as_slice(),
            "{} {}",
            fields[0],
            fields[1]
        );
        // The direct primitive publishes the same private header that its
        // carried error metadata exposes; reading the guest global is a
        // separate trace operation in the original source controls above.
        let direct = package(&mut vm, &["bad"]);
        assert_eq!(direct.code, tcl_runtime_api::Code::Error);
        let private = vm
            .native_return_error_code()
            .expect("published primitive code");
        let carried = crate::command::opt_get(&direct.options, "-errorcode").unwrap();
        assert_eq!(
            private.native_object_identity(),
            carried.native_object_identity()
        );
        compared += 1;
    }
    assert_eq!(compared, 21);
}

#[test]
fn package_completion_matches_all_six_native_84_controls() {
    const FIXTURES: [&str; 6] = [
        include_str!("../../../tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_package_ordinary/Jim.tsv"),
    ];
    const FILES: &[(&str, &[u8])] = &[
        (
            "/workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl",
            include_bytes!("../../../tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl")
                .as_slice(),
        ),
        (
            "/workspace/.proofs/2286-packages-dialects/package-native-completion/leaf.tcl",
            include_bytes!("../../../tcl-registry/tests/data/native_package_ordinary/leaf.tcl")
                .as_slice(),
        ),
        (
            "/workspace/.proofs/2286-packages-dialects/package-native-completion/nested.tcl",
            include_bytes!("../../../tcl-registry/tests/data/native_package_ordinary/nested.tcl")
                .as_slice(),
        ),
        (
            "/workspace/.proofs/2286-packages-dialects/package-native-completion/skip.tcl",
            include_bytes!("../../../tcl-registry/tests/data/native_package_ordinary/skip.tcl")
                .as_slice(),
        ),
    ];
    fn unhex(text: &str) -> Vec<u8> {
        assert_eq!(text.len() % 2, 0);
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let digits = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(digits, 16).unwrap()
            })
            .collect()
    }
    let jim: &'static tcl_dialect::DialectProfile =
        Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
    let mut compared = 0;
    for (engine, fixture) in FIXTURES.into_iter().enumerate() {
        for row in fixture.lines() {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            assert_eq!(fields.len(), 4);
            let name = fields[0];
            let script = unhex(fields[1]);
            let expected = unhex(fields[3]);
            let profile = if engine == 5 {
                jim
            } else {
                tcl_dialect::DialectProfile::find(TclVersion::ALL[engine].dialect_profile_name())
                    .unwrap()
            };
            let mut vm = crate::native_fixture::interpreter(profile);
            let host = tcl_test_support::fixed_sources::FixedSourceHost::new(vm.host_rc(), FILES);
            vm.set_host(std::rc::Rc::new(host));
            let completion = vm
                .eval_source(std::str::from_utf8(&script).unwrap())
                .expect("native package source");
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[2],
                "{engine} {name}"
            );
            let strings = vm.actual_native_string_protocol().unwrap();
            assert_eq!(
                completion
                    .result
                    .native_string_bytes(strings)
                    .unwrap()
                    .as_ref(),
                expected.as_slice(),
                "{engine} {name}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 84);
}
