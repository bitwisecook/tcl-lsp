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

//! FP-RBS-04, 05 and 08 and the call-by-name entries: qualified names, `namespace upvar` and `upvar` aliases.

use super::*;

// FP-RBS-04 — qualified-variable aliases (local name is the tail)

const FP_RBS_04_REPRO: &str = "\
proc ::ns::get {name key} {
    # `variable ${name}::graphAttr` declares the local alias 'graphAttr';
    # the qualified form is just where the storage lives.
    variable ${name}::graphAttr
    if {![info exists graphAttr($key)]} { return \"\" }
    return $graphAttr($key)
}
";

#[test]
fn fp_rbs_04_qualified_alias_silent() {
    // FP-RBS-04: `variable ${name}::tail` declares local alias — reads must not false-flag.
    assert!(
        !fires(FP_RBS_04_REPRO, D, "W210"),
        "FP-RBS-04: qualified-alias tail read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_04_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_04_REPRO, D, "W213"),
        "FP-RBS-04: qualified-alias tail read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_04_REPRO, D)
    );
}

#[test]
fn fp_rbs_04_static_qualified_alias_silent() {
    // FP: static-namespace form `variable ::ns::tail` also creates a tail-named local.
    let src = "\
proc tester {} {
    variable ::ns::children
    return $children
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-04: static-namespace alias tail must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-04: static-namespace alias tail must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_04_unrelated_tail_still_fires() {
    // TP control: a read of a tail name NOT declared as qualified alias must still fire W210.
    // `::ns` exists, or `proc` fails with `unknown namespace`.
    let src = "\
namespace eval ::ns {}
proc ::ns::get {name key} {
    variable ${name}::graphAttr
    # 'undeclared' has no alias decl and no set — must still fire W210.
    return $undeclared($key)
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-04 TP: $undeclared (no alias decl, no set) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-05 — namespace upvar alias-not-a-def

const FP_RBS_05_REPRO: &str = "\
proc tester {} {
    # tclsh: 'alias' is now the caller-scope name for ::ns::state.
    namespace upvar ::ns state alias
    return $alias
}
";

#[test]
fn fp_rbs_05_namespace_upvar_silent() {
    // FP-RBS-05: `namespace upvar ns src alias` is a real caller-scope def of `alias`.
    assert!(
        !fires(FP_RBS_05_REPRO, D, "W210"),
        "FP-RBS-05: namespace-upvar alias read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_05_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_05_REPRO, D, "W213"),
        "FP-RBS-05: namespace-upvar alias read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_05_REPRO, D)
    );
}

// `namespace upvar`'s otherVar may be an ARRAY ELEMENT.  tclsh 8.6 (and
// tclNamesp.c `NamespaceUpvarCmd`: the source resolves through
// `TclObjLookupVarEx(..., createPart1=1, createPart2=1)`, so an `(index)`
// suffix is parsed — and created — exactly as for `upvar`; `TclPtrMakeUpvar`
// then links the scalar local).  Verified live:
//   % namespace eval ns {variable arr; array set arr {k 1}}
//   % namespace upvar ns arr(k) local; puts $local        → 1
//   % proc t {} {namespace upvar ns arr(new) l; set l 7}
//   % t; puts $ns::arr(new)                                → 7
const FP_RBS_05_ELEM_REPRO: &str = "\
proc tester {} {
    # tclsh: 'local' is now the caller-scope name for ::ns::arr(k).
    namespace upvar ::ns arr(k) local
    return $local
}
";

#[test]
fn fp_rbs_05_namespace_upvar_array_element_silent() {
    // FP-RBS-05: an array-element source is as real a link as a scalar one.
    assert!(
        !fires(FP_RBS_05_ELEM_REPRO, D, "W210"),
        "FP-RBS-05: element-source alias read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_05_ELEM_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_05_ELEM_REPRO, D, "W213"),
        "FP-RBS-05: element-source alias read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_05_ELEM_REPRO, D)
    );
}

#[test]
fn fp_rbs_05_array_element_alias_write_only_silent() {
    // Adjacent FP guard: a write through the element-source alias lands in
    // ::ns::arr(k) — observable outside the frame (tclsh: `set l 7` above
    // creates/overwrites `ns::arr(new)`) — so it is neither a dead store
    // (W220) nor an unused variable (W211).
    let src = "proc t {} {\n    namespace upvar ::ns arr(k) local\n    set local 5\n}\n";
    assert!(
        !fires(src, D, "W220"),
        "FP-RBS-05: write through an element-source alias must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W211"),
        "FP-RBS-05: write through an element-source alias must NOT fire W211; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_05_array_element_unrelated_read_still_fires() {
    // TP control: the silence above comes from the alias binding of `local`,
    // not from a blanket namespace-upvar suppression — an unrelated,
    // never-written `$other` in the same proc must still fire W210.
    let src = "proc tester {} {\n    namespace upvar ::ns arr(k) local\n    return $other\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-05 TP: $other (no alias, no set) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_05_alias_exists_guard_is_not_constant() {
    // The `::safe::CheckInterp` guard shape (safe.tcl:109): whether an alias
    // local exists tracks the linked namespace variable — runtime state — so
    // `[info exists alias]` must not fold to a constant branch (I230).
    // tclsh 8.6: `namespace upvar ns s a; info exists a` → 1 when `ns::s`
    // exists, 0 when it does not.  Without the alias def the fold would say
    // "always false", since the lowered `namespace upvar` Call carries no defs
    // unlike `global`/`variable`/`upvar`.  Scalar and array-element sources both.
    for src in [
        "proc t {} {\n    namespace upvar ::ns state alias\n    if {[info exists alias]} { puts yes }\n}\n",
        "proc t {} {\n    namespace upvar ::ns arr(k) alias\n    if {[info exists alias]} { puts yes }\n}\n",
    ] {
        assert!(
            !fires(src, D, "I230"),
            "FP-RBS-05: alias existence guard must NOT fold to I230; emitted: {:?}",
            codes(src, D)
        );
    }
}

// FP-RBS-08 — upvar with a dynamic target (upvar 1 $name var) is a real alias-def

const FP_RBS_08_REPRO: &str = "\
proc f {name} {
    # picoirc.tcl:69 pattern: upvar 1 $context irc — aliases 'irc'
    # to whatever the caller named.  Writes + reads must be silent.
    upvar 1 $name var
    set var 99
    return $var
}
";

#[test]
fn fp_rbs_08_dynamic_upvar_target_silent() {
    // FP-RBS-08: `upvar 1 $name var` is a real alias-def even with dynamic target.
    let cs = codes(FP_RBS_08_REPRO, D);
    let rbs_or_ds = cs
        .iter()
        .any(|c| matches!(c.as_str(), "W210" | "W213" | "W214" | "W220" | "W211"));
    assert!(
        !rbs_or_ds,
        "FP-RBS-08: dynamic-target upvar alias use must NOT fire any RBS/dead-store/unused code; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_08_unrelated_local_still_fires() {
    // TP control: an unrelated local (not aliased, not set) must still fire W210.
    let src = "\
proc f {name} {
    upvar 1 $name var
    return $unrelated
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-08 TP: $unrelated (no alias, no set) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// call-by-name DYNAMIC_NAME_LOCAL trait tests

#[test]
fn fp_rbs_callbyname_scan_target_not_caller_alias() {
    // TP: scan $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src = "proc maybe {target} { scan 42 %d $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "scan $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_regexp_target_not_caller_alias() {
    // TP: regexp $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src =
        "proc maybe {target} { regexp {(.)} a -> $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "regexp $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_regsub_target_not_caller_alias() {
    // TP: regsub $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src = "proc maybe {target} { regsub a a b $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "regsub $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_lassign_target_not_caller_alias() {
    // TP: lassign $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src = "proc maybe {target} { lassign {1} $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "lassign $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_upvar_alias_still_suppresses() {
    // TP control: when callee uses `upvar $target v`, caller's x IS aliased — must NOT fire W211/W220.
    let src = "proc maybe {target} { upvar 1 $target v\n scan 42 %d v }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    let fires_any = cs.iter().any(|c| c == "W211" || c == "W220");
    assert!(
        !fires_any,
        "upvar-aliased callee write must continue to suppress caller W211/W220; emitted: {cs:?}"
    );
}
