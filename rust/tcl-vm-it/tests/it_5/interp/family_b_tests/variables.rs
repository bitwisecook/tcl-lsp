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

//! Variable links across `upvar`, `global`, `variable` and `namespace upvar`, under each release.

use crate::prelude::*;

#[test]
fn native_double_strings_observe_precision_only_until_first_materialisation() {
    let source = "set tcl_precision 12; set x 1.0; set d [expr {$x/3}]; set tcl_precision 4; set first [list $d]; set tcl_precision 12; set second [list $d]; set e [expr {$x/3}]; expr {$e+0}; set tcl_precision 4; set third [list $e]; list $first $second $third";
    for profile in variable_link_profiles() {
        let mut vm = crate::fixture::interpreter(profile);
        let wanted = match profile.double_string_policy().unwrap() {
            tcl_dialect::DoubleStringPolicy::Tcl84Precision
            | tcl_dialect::DoubleStringPolicy::TclPrecision => "0.3333 0.3333 0.3333",
            tcl_dialect::DoubleStringPolicy::Shortest => {
                "0.3333333333333333 0.3333333333333333 0.3333333333333333"
            }
            tcl_dialect::DoubleStringPolicy::JimTwelve => {
                "0.333333333333 0.333333333333 0.333333333333"
            }
        };
        let result = crate::fixture::eval(&mut vm, source);
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(result.result.to_str().as_ref(), wanted, "{}", profile.name);
    }
}

#[test]
fn native_scan_and_binary_fields_preserve_lazy_double_representations() {
    let cases = [
        (
            "set tcl_precision 4; set d [expr {1.0/3}]; binary scan [binary format d $d] d copy; set tcl_precision 12; list $d $copy",
            "0.333333333333 0.333333333333",
            "0.3333333333333333 0.3333333333333333",
        ),
        (
            "set tcl_precision 4; set d [expr {1.0/3}]; binary scan [binary format d* [list $d $d]] d* copy; set tcl_precision 12; list $d $copy",
            "0.333333333333 {0.333333333333 0.333333333333}",
            "0.3333333333333333 {0.3333333333333333 0.3333333333333333}",
        ),
        (
            "set tcl_precision 12; scan 0.3333333333333333 %f d; set tcl_precision 4; list $d",
            "0.3333",
            "0.3333333333333333",
        ),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, precision_wanted, shortest_wanted) in cases {
            let mut vm = crate::fixture::interpreter(profile);
            let wanted = match profile.double_string_policy().unwrap() {
                tcl_dialect::DoubleStringPolicy::Shortest => shortest_wanted,
                _ => precision_wanted,
            };
            let result = crate::fixture::eval(&mut vm, source);
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_observer_follows_storage_and_validates_writes() {
    let cases = [
        (
            "set tcl_precision 12; proc localPrecision {} {set tcl_precision 4; expr {1.0/3}}; localPrecision",
            "0.333333333333",
        ),
        (
            "set tcl_precision 4; upvar 0 tcl_precision alias; set alias 6; set status [catch {set tcl_precision 18} message]; list $status $message $tcl_precision",
            "1 {can't set \"tcl_precision\": improper value for precision} 6",
        ),
        (
            "set tcl_precision 4; set child [interp create -safe]; set status [$child eval {catch {set tcl_precision 8} message}]; set message [$child eval {set message}]; interp delete $child; list $status $message $tcl_precision",
            "1 {can't set \"tcl_precision\": can't modify precision from a safe interpreter} 4",
        ),
    ];
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, wanted) in cases {
            let mut vm = crate::fixture::interpreter(profile);
            let result = crate::fixture::eval(&mut vm, source);
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_array_trace_runs_before_element_callbacks() {
    let cases = [
        (
            "set tcl_precision 12; unset tcl_precision; set tcl_precision(x) 12; upvar 0 tcl_precision(x) p; set p 4; set d [expr {1.0/3}]; list $d $p $tcl_precision(x)",
            "0.333333333333 4 12",
        ),
        (
            "set tcl_precision 12; unset tcl_precision; set tcl_precision(x) 12; proc change {name key op} {upvar #0 $name a; set a($key) 8}; trace variable tcl_precision(x) w change; set tcl_precision(x) 4; list [expr {1.0/3}] $tcl_precision(x)",
            "0.3333 4",
        ),
    ];
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, wanted) in cases {
            let mut vm = crate::fixture::interpreter(profile);
            let result = crate::fixture::eval(&mut vm, source);
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_is_shared_with_children_and_survives_unset() {
    let source = "set tcl_precision 4; set x 1.0; set d [expr {$x/7}]; set child [interp create]; $child eval {set tcl_precision 12}; set result [list $tcl_precision $d]; interp delete $child; unset tcl_precision; list $result [info exists tcl_precision] $tcl_precision";
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut vm = crate::fixture::interpreter(profile);
        let result = crate::fixture::eval(&mut vm, source);
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(
            result.result.to_str().as_ref(),
            "{12 0.142857142857} 1 12",
            "{}",
            profile.name
        );
    }
}

#[test]
fn variable_links_follow_authored_dialect_lifetimes() {
    let cases = [
        (
            "set x OLD; upvar 0 x y; unset x; set y NEW; list $x $y",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "set x OLD; upvar 0 x y; unset x; set x NEW; list $x $y",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "set results {}; upvar 0 x y; foreach n {1 2 3} {set x $n; lappend results $y; unset x}; set results",
            "1 2 3",
            "1 2 3",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a; set a(k) NEW; list [catch {set y} error] $error $a(k)",
            "1 {can't read \"y\": no such variable} NEW",
            "0 NEW NEW",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a(k); set a(k) NEW; list $y $a(k)",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "namespace eval ::life {variable x OLD}; upvar #0 ::life::x y; namespace delete ::life; namespace eval ::life {variable x NEW}; list [catch {set y} error] $error $::life::x",
            "1 {can't read \"y\": no such variable} NEW",
            "0 NEW NEW",
        ),
        (
            "namespace eval ::life2 {variable x OLD}; upvar #0 ::life2::x y; unset ::life2::x; set y NEW; list $y $::life2::x",
            "NEW NEW",
            "NEW NEW",
        ),
    ];
    for profile in variable_link_profiles() {
        for (source, cell_want, name_want) in cases {
            let mut vm = variable_link_vm(profile);
            let wanted = match profile.variable_link_binding().unwrap() {
                tcl_dialect::VariableLinkBinding::StableCell => cell_want,
                tcl_dialect::VariableLinkBinding::SelectedFrameName => name_want,
            };
            let result = vm
                .eval_source(&format!("proc probe {{}} {{{source}}}; probe"))
                .unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                &*result.result.to_str(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn variable_link_target_rebinding_and_materialisation_follow_the_policy() {
    let cases = [
        (
            "set x OLD; set z NEW; upvar 0 x a; upvar 0 a b; upvar 0 z a; list $a $b",
            "0 {NEW OLD}",
            "0 {NEW NEW}",
        ),
        (
            "upvar 0 a(k) y; list [info exists a] [array exists a] [info exists y]",
            "0 {1 1 0}",
            "0 {0 0 0}",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a; set y NEW; list $a(k) $y",
            "1 {can't set \"y\": upvar refers to element in deleted array}",
            "0 {NEW NEW}",
        ),
        (
            "upvar 0 x x",
            "1 {can't upvar from variable to itself}",
            "1 {can't upvar from variable to itself}",
        ),
        (
            "upvar 0 a b; upvar 0 b a",
            "1 {can't upvar from variable to itself}",
            "1 {can't upvar from variable to itself}",
        ),
    ];
    for profile in variable_link_profiles() {
        for (source, cell_want, name_want) in cases {
            let mut vm = variable_link_vm(profile);
            let result = vm
                .eval_source(&format!(
                    "proc probe {{}} {{{source}}}; list [catch probe result] $result"
                ))
                .unwrap();
            let wanted = if profile.variable_link_binding()
                == Some(tcl_dialect::VariableLinkBinding::StableCell)
            {
                cell_want
            } else {
                name_want
            };
            assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
            assert_eq!(
                &*result.result.to_str(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

fn variable_link_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    let jim = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    assert_eq!(
        jim.core_point
            .map(tcl_dialect::model::DialectPoint::release),
        Some(tcl_dialect::model::Release::JIM_0_84),
        "the VM fixture and pinned native oracle must select the same release"
    );
    profiles.push(jim);
    profiles
}

/// The selected engine with the scripted distribution its shell loads, so
/// Jim's `namespace` helpers are present as they are under jimsh.
fn variable_link_vm(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    crate::fixture::distribution(profile)
}
