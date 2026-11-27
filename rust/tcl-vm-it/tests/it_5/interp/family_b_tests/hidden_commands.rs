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

//! Hidden and imported procedures across a change of release, and the
//! standard channels across a change of profile.

use super::*;

#[test]
fn only_a_changed_root_profile_reestablishes_standard_channel_defaults() {
    let mut vm = compiled_vm();
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    vm.set_dialect_profile(v90);

    assert_eq!(
        eval_value(
            &mut vm,
            "fconfigure stdout -encoding ascii -translation crlf -profile replace; \
                 list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace"
    );

    vm.set_dialect_profile(v90);
    assert_eq!(
        eval_value(
            &mut vm,
            "list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace",
        "re-pinning the root to the same profile reset live channel state"
    );

    eval_value(&mut vm, "interp create child");
    assert!(vm.set_child_dialect_profile("child", v86));
    assert_eq!(
        eval_value(
            &mut vm,
            "list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace",
        "pinning a child profile reset the VM-wide standard channel"
    );

    vm.set_dialect_profile(v86);
    assert_eq!(
        eval_value(&mut vm, "fconfigure stdout -translation"),
        "lf",
        "changing the root profile did not establish release defaults"
    );
}

/// In the interpreter `path` (`{}` for this one): define `p`, hide it as
/// `held` and define a visible replacement under 8.5, then select 8.6.
fn prepare_stale_hidden_proc(vm: &mut Vm, path: &str) {
    select(vm, path, "tcl8.5");
    eval_value(
        vm,
        &format!("interp eval {path} {{proc p {{}} {{ return hidden }}}}"),
    );
    eval_value(vm, &format!("interp hide {path} p held"));
    eval_value(
        vm,
        &format!("interp eval {path} {{proc p {{}} {{ return replacement }}}}"),
    );
    select(vm, path, "tcl8.6");
}

fn select(vm: &mut Vm, path: &str, name: &str) {
    if path == "{}" {
        select_release(vm, name);
    } else {
        select_child_release(vm, path, name);
    }
}

/// As [`prepare_stale_hidden_proc`], with an enter trace on the hidden
/// procedure that deletes it and defines the replacement.
fn prepare_trace_deleted_hidden_proc(vm: &mut Vm, path: &str) {
    select(vm, path, "tcl8.5");
    eval_value(
        vm,
        &format!(
            "interp eval {path} {{proc p {{}} {{ return hidden }}; \
             proc replace args {{ rename p {{}}; interp expose {{}} held p; rename p {{}}; proc p {{}} {{ return replacement }} }}; \
             trace add execution p enter replace}}"
        ),
    );
    eval_value(vm, &format!("interp hide {path} p held"));
    eval_value(
        vm,
        &format!("interp eval {path} {{proc p {{}} {{ return initial }}}}"),
    );
    select(vm, path, "tcl8.6");
}

#[test]
fn refreshed_hidden_proc_stays_hidden_and_preserves_visible_replacement() {
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_6);
    prepare_stale_hidden_proc(&mut vm, "{}");
    assert_eq!(eval_value(&mut vm, "interp invokehidden {} held"), "hidden");
    assert_eq!(eval_value(&mut vm, "p"), "replacement");
    assert_eq!(eval_value(&mut vm, "interp hidden {}"), "held");

    eval_value(&mut vm, "interp create child");
    prepare_stale_hidden_proc(&mut vm, "child");
    assert_eq!(
        eval_value(&mut vm, "interp invokehidden child held"),
        "hidden"
    );
    assert_eq!(eval_value(&mut vm, "child eval p"), "replacement");

    select_child_release(&mut vm, "child", "tcl8.5");
    assert_eq!(eval_value(&mut vm, "child invokehidden held"), "hidden");
    assert_eq!(eval_value(&mut vm, "child eval p"), "replacement");
    assert_eq!(eval_value(&mut vm, "interp hidden child"), "held");
}

#[test]
fn trace_deleted_hidden_refresh_never_resurrects_or_overwrites_replacement() {
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_6);
    prepare_trace_deleted_hidden_proc(&mut vm, "{}");
    let deleted = "1 {attempt to invoke a deleted command}";
    assert_eq!(
        eval_value(&mut vm, "list [catch {interp invokehidden {} held} r] $r"),
        deleted
    );
    assert_eq!(eval_value(&mut vm, "p"), "replacement");
    assert_eq!(eval_value(&mut vm, "interp hidden {}"), "");

    for (child, invoke) in [
        ("a", "interp invokehidden a held"),
        ("b", "b invokehidden held"),
    ] {
        eval_value(&mut vm, &format!("interp create {child}"));
        prepare_trace_deleted_hidden_proc(&mut vm, child);
        assert_eq!(
            eval_value(&mut vm, &format!("list [catch {{{invoke}}} r] $r")),
            deleted,
            "{invoke}"
        );
        assert_eq!(
            eval_value(&mut vm, &format!("{child} eval p")),
            "replacement"
        );
        assert_eq!(eval_value(&mut vm, &format!("interp hidden {child}")), "");
    }
}

#[test]
fn relocated_hidden_refresh_persists_at_the_live_visible_key() {
    let mut vm = tcl_vm::test_support::native_vm();
    select_release(&mut vm, "tcl8.5");
    eval_value(&mut vm, "proc p {} { return hidden }");
    eval_value(
        &mut vm,
        "proc expose args { rename p {}; interp expose {} held p; trace remove execution p enter expose }",
    );
    eval_value(&mut vm, "trace add execution p enter expose");
    eval_value(&mut vm, "interp hide {} p held");
    eval_value(&mut vm, "proc p {} { return replacement }");
    select_release(&mut vm, "tcl8.6");

    assert_eq!(eval_value(&mut vm, "interp invokehidden {} held"), "hidden");
    assert_eq!(eval_value(&mut vm, "interp hidden {}"), "");
    assert_eq!(eval_value(&mut vm, "p"), "hidden");
}

#[test]
fn imported_proc_profile_refresh_updates_the_resolved_import_not_its_source_name() {
    let mut vm = tcl_vm::test_support::native_vm();
    select_release(&mut vm, "tcl8.5");
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval src {proc p {} {return original}; namespace export p}; \\
                 namespace eval dst {namespace import ::src::p}; \\
                 rename ::src::p ::src::q; proc ::src::p {} {return replacement}",
        ),
        ""
    );

    select_release(&mut vm, "tcl8.6");
    assert_eq!(eval_value(&mut vm, "::dst::p"), "original");
    assert_eq!(eval_value(&mut vm, "::src::p"), "replacement");
    assert_eq!(eval_value(&mut vm, "::src::q"), "original");
}
