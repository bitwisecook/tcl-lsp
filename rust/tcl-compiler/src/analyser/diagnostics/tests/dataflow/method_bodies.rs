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

//! W210 and W211 inside `TclOO`, snit and itcl method bodies.

use super::*;

// TclOO/snit method-body CFG/SSA diagnostics

#[test]
fn method_body_read_before_set_now_flags_w210() {
    // TP — reduced from `nico-robert_tomato`'s
    // Vector3d.tcl: `method * {type}` reads `$other`, a variable belonging to
    // a sibling method and never bound in its own scope. The fixture uses
    // absolute command heads to isolate that Tcl 9.0.4 read-before-set fact
    // from runtime-selected method-relative command lookup.
    let src = "oo::class create Vector3d {\n    variable _x\n    constructor {x} { ::set _x $x }\n    method DotProduct {other} { ::return [::expr {$_x * $other}] }\n    method Buggy {type} { ::return [::expr {$_x * $other}] }\n}\n";
    let codes = codes_for_in(src, "tcl9.0");
    assert_eq!(
        codes,
        vec!["W210".to_string()],
        "the unbound `$other` read inside Buggy must flag exactly once: {codes:?}"
    );
}

#[test]
fn method_body_instance_variable_read_does_not_false_positive_w210() {
    // FP guard — `variable _x` declared at class level auto-binds `_x` in
    // *every* method's scope with no visible `variable` statement in the
    // method body itself (real TclOO semantics); a naive fix that just
    // iterates `cu.methods` without threading `MethodDef::instance_vars`
    // into the read-before-set suppression set would flood a false W210 on
    // every ordinary instance-variable read — this is that read, alone,
    // with no sibling bug present.
    let src = "oo::class create P {\n    variable _x\n    constructor {x} { set _x $x }\n    method X {} { return $_x }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "a legitimate instance-variable read must not flag W210: {codes:?}"
    );
}

#[test]
fn method_body_own_parameter_read_does_not_false_positive_w210() {
    // FP guard — a method's *own* declared parameter must not flag W210
    // either. `emit_read_before_set_diagnostics` / `emit_return_phi_undef_w210`
    // both special-case a real parameter via a separate `ir_module.procedures`
    // lookup keyed by the function's qualified name, which a method's
    // qualified name is never in (methods live in `ir_module.methods`, a
    // different map) — so without also folding `MethodDef::params` into the
    // suppression set, every method parameter would falsely read-before-set
    // (threading only `instance_vars` still flags `other` in `DotProduct
    // {other}` itself, and `x` in the constructor).
    let src =
        "oo::class create P {\n    method DotProduct {other} { return [expr {1 * $other}] }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "a method's own used parameter must not flag W210: {codes:?}"
    );
}

#[test]
fn method_body_setter_write_with_no_local_read_does_not_false_positive_dead_store() {
    // FP guard — a "setter" method that writes an instance variable with no
    // local read is not a dead store (W220) or unused variable (W211):
    // another method reads the value later. Mirrors the pre-existing
    // cross-function-global suppression (`globals_written_by_procs`), just
    // object-instance-scoped.
    let src = "oo::class create P {\n    variable _x\n    method SetX {v} { set _x $v }\n    method GetX {} { return $_x }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W220".to_string()) && !codes.contains(&"W211".to_string()),
        "a setter's instance-variable write must not flag dead-store/unused: {codes:?}"
    );
}

#[test]
fn method_body_ordinary_unused_local_still_flags_w211() {
    // TN — the fix must not blanket-suppress every diagnostic inside a
    // method body: a genuinely unused *local* (not a param, not an instance
    // var) still flags W211, exactly as it would inside a plain proc.
    let src = "oo::class create P {\n    method Foo {} {\n        ::set unused 1\n        ::return ok\n    }\n}\n";
    let codes = codes_for_in(src, "tcl8.6");
    assert!(
        codes.contains(&"W211".to_string()),
        "a genuinely unused local inside a method must still flag W211: {codes:?}"
    );
}

// Method units come from the registry definer grammars, so
// `oo::objdefine`, snit, and itcl method bodies get the same CFG/SSA
// diagnostic family.

#[test]
fn objdefine_method_body_unbound_read_now_flags_w210() {
    // An `oo::objdefine $obj { method … }` body must produce a method unit,
    // or a genuinely unbound read inside it draws no diagnostics at all.
    let src = "oo::class create C {}\nset k [C new]\noo::objdefine $k {\n    method probe {} { ::return $neverBound }\n}\n";
    let codes = codes_for_in(src, "tcl8.6");
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read inside an objdefine method must flag W210: {codes:?}"
    );
}

#[test]
fn objdefine_per_object_variable_does_not_false_positive_w210() {
    // FP guard — the objdefine block's own `variable z` auto-binds `z` in
    // the per-object method's frame exactly as a class-level declaration
    // does for class methods; reading it must not flag.
    let src = "oo::class create C {}\nset k [C new]\noo::objdefine $k {\n    variable z\n    method probe {} { if {[info exists z]} { return $z }\nreturn {} }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "a per-object instance-variable read must not flag W210: {codes:?}"
    );
}

#[test]
fn snit_method_body_unbound_read_now_flags_w210() {
    // A snit method body must become a method unit, or this unbound read is
    // invisible.
    let src = "snit::type Dog {\n    method bark {} { return $neverBound }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read inside a snit method must flag W210: {codes:?}"
    );
}

#[test]
fn snit_method_implicits_and_declared_vars_do_not_false_positive_w210() {
    // FP guard — snit injects `self` / `selfns` / `type` / `options` into
    // every method body, and a type-level `variable` declaration is
    // auto-linked; none of those reads may flag.
    let src = "snit::type Dog {\n    variable name Name\n    option -color white\n    method describe {} { return \"$self $type $name $options(-color)\" }\n}\n";
    let codes = crate::provider_fixtures::analyse(
        &format!("package require snit\n{src}"),
        "tcl8.6",
        &[crate::provider_fixtures::Provider::Snit],
    )
    .diagnostics
    .into_iter()
    .map(|diagnostic| diagnostic.code.to_string())
    .collect::<Vec<_>>();
    assert!(
        !codes.contains(&"W210".to_string()),
        "snit implicit / declared variable reads must not flag W210: {codes:?}"
    );
}

#[test]
fn snit_widget_win_and_hull_do_not_false_positive_w210() {
    // FP guard — the widget definers additionally inject `win` / `hull`
    // (registry data: SNIT_WIDGET_GRAMMAR's implicit_vars).
    let src = "snit::widget Bar {\n    method redraw {} { return \"$win $hull\" }\n}\n";
    let codes = crate::provider_fixtures::analyse(
        &format!("package require snit\n{src}"),
        "tcl8.6",
        &[crate::provider_fixtures::Provider::Snit],
    )
    .diagnostics
    .into_iter()
    .map(|diagnostic| diagnostic.code.to_string())
    .collect::<Vec<_>>();
    assert!(
        !codes.contains(&"W210".to_string()),
        "widget `win`/`hull` reads must not flag W210: {codes:?}"
    );
}

#[test]
fn itcl_method_body_unbound_read_now_flags_w210() {
    // An itcl method body must become a method unit too.
    let src = "itcl::class Toaster {\n    public method toast {} { return $neverBound }\n}\n";
    let codes = codes_for(src);
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read inside an itcl method must flag W210: {codes:?}"
    );
}

#[test]
fn itcl_instance_state_and_this_do_not_false_positive_w210() {
    // FP guard — itcl auto-binds `this`, every `variable`, and every
    // `common` in method bodies (including modifier-wrapped declarations).
    let src = "itcl::class Toaster {\n    variable crumbs 0\n    protected common heat 3\n    public method status {} { return \"$this $crumbs $heat\" }\n}\n";
    let codes = codes_for(src);
    assert!(
        !codes.contains(&"W210".to_string()),
        "itcl `this`/variable/common reads must not flag W210: {codes:?}"
    );
}

#[test]
fn method_body_unbound_read_inside_switch_arm_still_flags_w210() {
    // TN — a sanity check that the new loop reaches a method's full CFG,
    // not just a shallow top-level scan: the unbound read is nested inside
    // a `switch` arm.
    let src = "oo::class create P {\n    method Foo {n} {\n        ::switch $n {\n            1 { ::return $missing }\n        }\n        ::return 0\n    }\n}\n";
    let codes = codes_for_in(src, "tcl8.6");
    assert!(
        codes.contains(&"W210".to_string()),
        "an unbound read nested inside a switch arm must still flag W210: {codes:?}"
    );
}
