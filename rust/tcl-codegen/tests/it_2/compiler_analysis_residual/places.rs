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

//! Def and read places: dict and array writes, aliases, traces, terminators and data words that are not reads.

use super::*;

// place_bridge.rs — place/SSA-bridge def/read resolution

#[test]
fn place_dict_set_defs_variable_and_dollar_read_observes_it() {
    // `dict set d a b 1` writes the WHOLE dict variable `d` (the place bridge
    // resolves the VAR_WRITE target at variable granularity — a Scalar `d`, not
    // an element path: the dict commands mutate the bound variable in place).
    // The later `[dict get $d a b]` reads `$d`, which overlaps that def.
    let (defs, reads) = places_of(
        "proc f {} { dict set d a b 1; puts [dict get $d a b] }",
        "::f",
    );
    let d = place::scalar("d", LOCAL_NS, false);
    assert!(
        defs.iter().any(|p| p.name == "d"),
        "dict set must def the variable d, got {defs:?}",
    );
    assert!(
        reads.iter().any(|r| overlap(r, &d)),
        "the `$d` read in `dict get $d a b` must observe d, reads={reads:?}",
    );
}

#[test]
fn place_must_alias_kill_suppresses_dead_element() {
    // `set a(k) 1; set a(k) 2; puts $a(k)` — the FIRST write to a(k) is dead
    // (a later write to the exact same literal key with no intervening read),
    // even though a read later observes a(k).  So a(k) is NOT in the
    // "observed" suppression set for the first def's block/index, but the
    // SECOND write IS observed.  We assert the must-alias machinery via the
    // public `element_writes_observed_by_reads`: it returns the indices of the
    // element writes a read observes, EXCLUDING must-alias-killed ones.
    let r = reg();
    let cu = CompilationUnit::build_for(
        "proc f {} { set a(k) 1; set a(k) 2; puts $a(k) }",
        &r,
        false,
    );
    let fu = cu.fixture_function("::f").expect("proc");
    let observed = element_writes_observed_by_reads(&fu.cfg, &fu.name, &r);
    // Exactly one element write is observed-and-kept (the second `set a(k) 2`);
    // the first is must-alias-killed so it is excluded.
    assert_eq!(
        observed.len(),
        1,
        "only the surviving a(k) write is observed (first is must-alias killed); got {observed:?}",
    );
}

#[test]
fn place_array_element_write_no_kill_is_observed() {
    // Control: distinct keys (`a(k)` then `a(j)`), each read — both are
    // observed (no must-alias kill since the keys differ).
    let r = reg();
    let cu = CompilationUnit::build_for(
        "proc f {} { set a(k) 1; set a(j) 2; puts $a(k); puts $a(j) }",
        &r,
        false,
    );
    let fu = cu.fixture_function("::f").expect("proc");
    let observed = element_writes_observed_by_reads(&fu.cfg, &fu.name, &r);
    assert_eq!(
        observed.len(),
        2,
        "both distinct-key element writes are observed; got {observed:?}",
    );
}

#[test]
fn place_no_array_assignment_is_empty() {
    // The cheap pre-check: a function with no array-element assignment returns
    // an empty observed set without scanning.
    let r = reg();
    let cu = CompilationUnit::build_for("proc f {} { set x 1; puts $x }", &r, false);
    let fu = cu.fixture_function("::f").expect("proc");
    assert!(
        element_writes_observed_by_reads(&fu.cfg, &fu.name, &r).is_empty(),
        "no array assignment → empty observed set",
    );
}

#[test]
fn place_braced_literal_data_word_is_not_a_read() {
    // `puts {$a(k)}` is a BRACED literal data word — Tcl performs no
    // substitution, so it must NOT register a read of a(k).  A real dead store
    // to a(k) would otherwise be wrongly suppressed. tclsh 8.4.20 and 9.1.0
    // print `$a(k)` for `f`.
    let (_defs, reads) = places_of("proc f {} { set a(k) 1; puts {$a(k)} }", "::f");
    let ak = place::array_elem("a", Index::literal("k"), LOCAL_NS, false);
    assert!(
        !reads.iter().any(|r| overlap(r, &ak)),
        "a braced literal {{$a(k)}} must not read a(k); reads={reads:?}",
    );
}

#[test]
fn place_quoted_data_word_does_read() {
    // Control: a double-quoted word DOES substitute, so `puts "$a(k)"` reads
    // a(k).
    let (_defs, reads) = places_of("proc f {} { set a(k) 1; puts \"$a(k)\" }", "::f");
    let ak = place::array_elem("a", Index::literal("k"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &ak)),
        "a quoted \"$a(k)\" must read a(k); reads={reads:?}",
    );
}

#[test]
fn place_namespace_upvar_alias_resolves() {
    // `namespace upvar ::ns v local` binds `local` to the namespace var
    // ::ns::v: `local` resolves to that variable's own cell, not a fresh
    // local scalar. tclsh 8.5.19 to 9.1.0 print 1 for `puts $::ns::v` after
    // `f` (8.4.20 has no `namespace upvar`).
    let r = reg();
    let src = "namespace eval ns { variable v 0 }\nproc f {} { namespace upvar ::ns v local; set local 1 }";
    let cu = CompilationUnit::build_for(src, &r, false);
    let fu = cu.fixture_function("::f").expect("proc");
    let ctx = context_before(fu, src, "set local 1");
    let p = tcl_compiler::var_resolve::resolve_place("local", ctx, false, &r);
    assert_eq!(
        (p.kind, p.ns.as_str(), p.name.as_str(), p.dynamic),
        (PlaceKind::Scalar, "::ns", "v", false),
        "the namespace-upvar local is ::ns::v: {p:?}",
    );
}

#[test]
fn place_trace_target_marks_observed() {
    // `trace add variable g write …` marks `g` as traced → externally
    // observable, so a write to it is never dead.
    let r = reg();
    let src = "proc f {} { set g 1; trace add variable g write cb; set g 2 }";
    let cu = CompilationUnit::build_for(src, &r, false);
    let fu = cu.fixture_function("::f").expect("proc");
    let ctx = context_before(fu, src, "set g 2");
    // The traced set holds the cell `g` names in this activation.
    let g = tcl_compiler::var_resolve::resolve_place("g", ctx, false, &r);
    assert!(
        ctx.traced
            .contains(&tcl_compiler::var_resolve::cell_key(&g)),
        "trace add variable should record g as traced; g={g:?} traced={:?}",
        ctx.traced,
    );
    assert!(g.observed, "a traced g is observed: {g:?}");
}

#[test]
fn place_whole_array_read_overlaps_every_element() {
    // `array get arr` reads the WHOLE array → overlaps any element a(k).
    let (_defs, reads) = places_of("proc f {} { set out [array get arr] }", "::f");
    let arr_k = place::array_elem("arr", Index::literal("k"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &arr_k)),
        "array get arr reads the whole array; reads={reads:?}",
    );
}

#[test]
fn place_dynamic_var_read_name_is_unknown_top() {
    // `array get $name` — the read target is a runtime-computed name, so it
    // resolves to UNKNOWN/top, which overlaps everything (a write to any array
    // is therefore not falsely dead).
    let (_defs, reads) = places_of(
        "proc f {} { set name arr; set out [array get $name] }",
        "::f",
    );
    let any_elem = place::array_elem("whatever", Index::literal("z"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &any_elem)),
        "a dynamic array-name read must be UNKNOWN/top (overlaps all); reads={reads:?}",
    );
}

#[test]
fn place_terminator_branch_condition_reads() {
    // An `if {$cond} …` branch terminator reads `cond` (the condition), and a
    // `[expr]`-embedded command-subst within the condition is recursed.
    let r = reg();
    let cu = CompilationUnit::build_for("proc f {c} { if {$c} { puts yes } }", &r, false);
    let fu = cu.fixture_function("::f").expect("proc");
    let ctx = build_resolve_context(&fu.cfg, &fu.name);
    let mut term_reads = Vec::new();
    for block in fu.cfg.blocks.values() {
        if let Some(term) = &block.terminator {
            term_reads.extend(terminator_read_places(term, &ctx, &r));
        }
    }
    assert!(
        term_reads
            .iter()
            .any(|r| overlap(r, &place::scalar("c", LOCAL_NS, false))),
        "the branch condition must read c; term_reads={term_reads:?}",
    );
}

#[test]
fn place_terminator_braced_return_is_not_a_read() {
    // A braced `return {$x}` is a literal — its `$x` is not a read.  A bare
    // `return $x` IS a read.  We compare the two terminator-read sets.
    let r = reg();
    let braced = CompilationUnit::build_for("proc f {} { set x 1; return {$x} }", &r, false);
    let bare = CompilationUnit::build_for("proc g {} { set x 1; return $x }", &r, false);
    let read_x = |cu: &CompilationUnit, q: &str| -> bool {
        let fu = cu.fixture_function(q).expect("proc");
        let ctx = build_resolve_context(&fu.cfg, &fu.name);
        fu.cfg.blocks.values().any(|b| {
            b.terminator.as_ref().is_some_and(|t| {
                terminator_read_places(t, &ctx, &r)
                    .iter()
                    .any(|r| overlap(r, &place::scalar("x", LOCAL_NS, false)))
            })
        })
    };
    assert!(
        !read_x(&braced, "::f"),
        "braced return {{$x}} must not read x"
    );
    assert!(read_x(&bare, "::g"), "bare return $x must read x");
}

#[test]
fn place_incr_reads_its_own_target() {
    // `incr counter` reads (and writes) `counter`; the amount word can read
    // more.
    let (defs, reads) = places_of("proc f {} { set counter 0; incr counter }", "::f");
    let c = place::scalar("counter", LOCAL_NS, false);
    assert!(defs.iter().any(|d| overlap(d, &c)), "incr defs counter");
    assert!(
        reads.iter().any(|r| overlap(r, &c)),
        "incr reads its own target counter; reads={reads:?}",
    );
}

#[test]
fn place_block_body_reads_recovered() {
    // An `eval {puts $x}` lowers to a Block whose body runs in the enclosing
    // scope — its `$x` read must be recovered so the store to x isn't dead.
    let (_defs, reads) = places_of("proc f {} { set x 1; eval {puts $x} }", "::f");
    let x = place::scalar("x", LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &x)),
        "eval {{puts $x}} block body must read x; reads={reads:?}",
    );
}
