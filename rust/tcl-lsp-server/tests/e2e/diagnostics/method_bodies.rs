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

//! Method bodies are analysed end to end: `TclOO`, `oo::objdefine`, snit and itcl.

use super::*;

/// `Vector3d.tcl`'s `method * {type}` reads `$other`, a variable
/// belonging to a *sibling* method (`DotProduct {other}`), never bound in
/// `*`'s own scope — tclsh 8.6/9.0.4 both crash with `can't read "other":
/// no such variable` the instant `*` runs on an object operand. The entire
/// CFG/SSA dataflow diagnostic family (W210 read-before-set and its
/// siblings) must run on every `TclOO`/snit method body, not just plain
/// `proc` bodies — the per-function diagnostic loop must iterate
/// `cu.methods` as well as `cu.procedures`. The identical unbound-read
/// shape inside a plain `proc` already fires W210.
#[test]
fn method_body_unbound_sibling_parameter_read_flags_w210_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Vector3d {\n    variable _x\n    constructor {x} { ::set _x $x }\n    method DotProduct {other} { ::return [::expr {$_x * $other}] }\n    method Buggy {type} { ::return $other }\n}\n",
    );
    assert!(
        has_code(&diags, "W210"),
        "the unbound `$other` read inside Buggy must flag: {:?}",
        codes(&diags)
    );
}

/// FP guard: the check must not flood false W210s on ordinary
/// instance-variable reads or a method's own parameters —
/// `TclOO` auto-binds class-level `variable` declarations in every
/// method's scope with no visible `variable` statement in the body itself.
#[test]
fn method_body_instance_variable_and_own_parameter_reads_stay_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create P {\n    variable _x\n    constructor {x} { set _x $x }\n    method DotProduct {other} { return [expr {$_x * $other}] }\n}\n",
    );
    assert!(
        !has_code(&diags, "W210"),
        "legitimate instance-variable / own-parameter reads must not flag: {:?}",
        codes(&diags)
    );
}

// Lowering's OO extraction is driven by the registry definer grammars, so
// `oo::objdefine`, snit, and itcl method bodies must be analysed end to
// end and produce diagnostics like any other method body.

/// TP: an unbound read inside an `oo::objdefine` method body reaches the
/// client as W210.
#[test]
fn objdefine_method_body_w210_reaches_the_client() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create C {}\n\
         set k [C new]\n\
         oo::objdefine $k {\n\
             method probe {} { ::return $neverBound }\n\
         }\n",
    );
    assert!(
        has_code(&diags, "W210"),
        "objdefine method body W210 must publish: {:?}",
        codes(&diags)
    );
}

/// FP guard: the objdefine block's own per-object `variable` binds the name
/// in the per-object method's frame — no W210.
#[test]
fn objdefine_per_object_variable_read_publishes_no_w210() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create C {}\n\
         set k [C new]\n\
         oo::objdefine $k {\n\
             variable z\n\
             method probe {} { if {[info exists z]} { return $z }\nreturn {} }\n\
         }\n",
    );
    assert!(
        !has_code(&diags, "W210"),
        "per-object variable read must not flag: {:?}",
        codes(&diags)
    );
}

// Per-object reporting tables grant no current Native table closure.

/// A source construction report cannot prove Native per-object member absence.
#[test]
fn native_objdefine_retraction_of_undeclared_member_withdraws_w315() {
    // naming.diagnostics.original-object-abort-admission
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create C { method ghost {} { return g } }\n\
         set k [C new]\n\
         oo::objdefine $k { deletemethod ghost }\n",
    );
    assert!(
        !has_code(&diags, "W315"),
        "unproved Native per-object abort must be withdrawn: {:?}",
        codes(&diags)
    );
}

/// TN (CRITICAL FP guard): the cross-block shape that kept W315 out of
/// `oo::objdefine` — a second block retracting what the first declared is
/// legal, and the seeded walk must stay silent.
#[test]
fn objdefine_cross_block_retraction_publishes_no_w315() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create C {}\n\
         set k [C new]\n\
         oo::objdefine $k { method im {} { return i } }\n\
         oo::objdefine $k { deletemethod im }\n",
    );
    assert!(
        !has_code(&diags, "W315"),
        "retracting a member an earlier block declared is legal: {:?}",
        codes(&diags)
    );
}

/// TP + FP guard in one: a snit method body is analysed (unbound read
/// flags) while the injected implicits (`self`/`type`/`options`) and the
/// declared type variable stay clean.
#[test]
fn snit_method_bodies_are_analysed_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "snit::type Dog {\n\
             variable name\n\
             method bark {} { return $neverBound }\n\
             method describe {} { return \"$self $type $name\" }\n\
         }\n",
    );
    assert_eq!(
        on_line(&diags, "W210"),
        [2].into_iter().collect::<BTreeSet<i64>>(),
        "exactly the unbound read must flag: {:?}",
        codes(&diags)
    );
}

/// TP: an itcl method body is analysed end to end; `this` and declared
/// instance/common variables stay clean.
#[test]
fn itcl_method_bodies_are_analysed_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "itcl::class Toaster {\n\
             variable crumbs 0\n\
             public method toast {} { return $neverBound }\n\
             public method status {} { return \"$this $crumbs\" }\n\
         }\n",
    );
    assert_eq!(
        on_line(&diags, "W210"),
        [2].into_iter().collect::<BTreeSet<i64>>(),
        "exactly the unbound read must flag: {:?}",
        codes(&diags)
    );
}
