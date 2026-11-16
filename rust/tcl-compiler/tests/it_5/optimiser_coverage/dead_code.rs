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

//! O107 to O109 and O112: unreachable branches, aggressive dead code, dead stores and structure elimination, with their trace guards.

use super::*;

#[test]
fn o112_constant_branch_elimination_blocks_on_variable_trace() {
    // `structure_elimination`'s O112 (`if {K}` → replace with the
    // constant-true clause, dropping the `if`/condition entirely) reads
    // its own SCCP-lattice projection (`sccp_env_for`) — a second,
    // independent consumer of the same unsafe SCCP data `O100`'s branch
    // path above reads. Collapsing `if {$x} {...} else {...}` to just the
    // `puts yes` branch would delete the runtime read of `$x` outright,
    // permanently silencing the trace. tclsh: prints "trace fired" then
    // "yes" on every run — the `if` must stay a real runtime check so the
    // trace keeps firing.
    // Note: the `else` body's *contents* (`puts no`) must not be lost
    // either — see `o107_dead_code_elimination_of_unreachable_branch`
    // below, which asserts they survive (SCCP itself is trace-safe, so
    // every trace-blind consumer inherits correctness). This test only
    // asserts O112 itself does not collapse the `if` structure.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O112"));
    assert!(optimised(src, TCL).contains("if {$x}"));
}

#[test]
fn o107_dead_code_elimination_of_unreachable_branch() {
    // `elimination.rs`'s O107 "unreachable code" deletion is a
    // consumer of SCCP's `executable_blocks`/`constant_branches` facts, same
    // as `propagation`/`branch_folding`/`structure_elimination` above. A
    // consumer that independently rediscovers the "provably unreachable"
    // `else` body (once O112 stops collapsing the `if` structure itself)
    // and deletes its *contents* on a later pass would silently lose the
    // same trace-firing behaviour through a different code path. SCCP's
    // own dataflow (`rust/tcl-compiler/src/sccp.rs`) must instead treat a
    // read of a traced/aliased variable as `Overdefined` — via the
    // whole-module `Module::traced_variables` fact, which also catches a
    // trace installed by a *called* proc like this — so every consumer
    // inherits correctness for free: `if {$x}` is not provably constant,
    // so O107 must not fire and both arms survive. tclsh: prints "trace
    // fired" then "yes" —
    // the `else` body never runs, but it must still be present in the
    // rewritten source since the compiler cannot prove that statically.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O107"));
    assert!(optimised(src, TCL).contains("puts no"));
}

#[test]
fn o107_true_positive_untraced_branch_still_eliminated() {
    // TP / precision guard: the trace-safety fix above must not make dead
    // code universally survive — the identical shape with no trace anywhere
    // in the module is still provably unreachable and the `else` content is
    // still eliminated. (The identical-shaped structure folds via O112 —
    // higher optimiser priority — which collapses the whole `if`, so O107
    // itself never gets a separate "unreachable else block" to delete here;
    // observable behaviour, not the specific code, is what this guards.)
    // tclsh: prints "yes" only; `puts no` never runs and the rewritten
    // source drops it.
    let src = "set x 1\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(!optimised(src, TCL).contains("puts no"));
}

#[test]
fn o107_false_positive_guard_unrelated_variable_name_still_eliminated() {
    // FP guard: a trace on a *different*, merely similarly-named variable
    // (`x2`, sharing the `x` prefix) must not spuriously widen `x`'s own
    // branch to Overdefined — the registry-driven `traced_variables` lookup
    // is an exact-name set membership test, not a prefix/substring match.
    // (See the TP test above for why this folds via O112, not O107, once
    // the trace-guard doesn't apply.) tclsh: prints "yes" only.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x2 read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(!optimised(src, TCL).contains("puts no"));
}

#[test]
fn o107_dynamic_variable_trace_target_blocks_elimination() {
    // FN guard: a *non-literal* trace target (`trace add variable $name
    // ...`) exercises `Module::has_dynamic_variable_trace` rather than the
    // literal `traced_variables` set — a distinct code path in
    // `is_externally_mutable` that must widen *every* variable, not just
    // named ones, exactly like `crate::optimiser_coverage::gvn::is_pure_command_with_traces`'s
    // `has_dynamic_trace` handling. tclsh: prints "trace fired" then "yes"
    // — `else { puts no }` never runs but must still survive since the
    // compiler cannot enumerate which name(s) `$name` could be.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {name} {\n    trace add variable $name read onread\n}\nset x 1\nsetup ::x\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O107"));
    assert!(optimised(src, TCL).contains("puts no"));
}

// O107 / O108 / O109 — dead code & dead store elimination

#[test]
fn o107_dead_code_in_false_branch() {
    // `if {0}` body is unreachable; trailing statement survives.
    let out = optimised("if {0} {\n    puts dead\n}\nputs alive\n", TCL);
    assert!(!out.contains("puts dead"));
    assert!(out.contains("puts alive"));

    // Code after `return` in a proc: must not crash (O107 may or may not be
    // emitted). Assert only that optimisation completes (proc preserved).
    let after_ret = "proc foo {} {\n    return 1\n    puts never\n}\n";
    let _ = opt_codes(after_ret, TCL); // no panic
}

#[test]
fn o108_aggressive_dce() {
    // tclsh: a=1, b reassigned to 10 ⇒ b==10. The dead chain is removed via
    // O108/O109 (either code may carry it).
    let s = "set a 1\nset b [expr {$a + 1}]\nset b 10\nputs $b";
    assert!(optimised(s, TCL).contains("puts 10"));
    assert!(opt_any_of(s, TCL, &["O108", "O109"]));

    // A consumed value is not eliminated. tclsh: a=1 ⇒ b==2 ⇒ `puts 2`.
    assert!(optimised("set a 1\nset b [expr {$a + 1}]\nputs $b", TCL).contains("puts 2"));

    // Multi-step transitively-dead chain. tclsh: final c==99 ⇒ `puts 99`.
    let multi = "set a 1\nset b [expr {$a + 1}]\nset c [expr {$b + 1}]\nset c 99\nputs $c\n";
    assert!(optimised(multi, TCL).contains("puts 99"));
    assert!(opt_any_of(multi, TCL, &["O108", "O109"]));
}

#[test]
fn o109_dead_store_elimination() {
    // First store dead (O109). tclsh: a==2.
    let s = "set a 1\nset a 2\nputs $a";
    assert!(opt_fires(s, TCL, "O109"));
    assert!(optimised(s, TCL).contains("puts 2"));

    // tclsh: x final 200.
    assert!(optimised("set x 100\nset x 200\nputs $x", TCL).contains("puts 200"));

    // eval reads all vars — the store feeding `[eval $s]` survives (not DSE'd).
    let esc = "set n [eval $s]\nset n 1\nputs $n";
    assert_eq!(
        optimised(esc, TCL).lines().next().unwrap(),
        "set n [eval $s]"
    );

    // A dead store inside a proc fires O109.
    assert!(opt_fires(
        "proc foo {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
        TCL,
        "O109"
    ));

    // Call-by-name suppression: a local passed BY NAME to a proc with upvar
    // VAR_READ/VAR_WRITE traits is indirectly read — its store must survive
    // (no O109/O126). Deleting it would feed an uninitialised name to the callee.
    let byname = "proc reader {name} { upvar 1 $name local; puts $local }\nproc f {} {\n    set x \"hello\"\n    reader x\n}\n";
    assert!(opt_absent(byname, TCL, "O109"));
    assert!(opt_absent(byname, TCL, "O126"));

    // TP control: a dead store on a DIFFERENT (not passed-by-name) var still
    // fires O109.
    let byname_tp = "proc reader {name} { upvar 1 $name local; puts $local }\nproc f {} {\n    set unrelated \"discarded\"\n    set unrelated \"actual\"\n    set y \"passed\"\n    reader y\n    return $unrelated\n}\n";
    assert!(opt_fires(byname_tp, TCL, "O109"));
}

// O112 — structure elimination

#[test]
fn o112_if_elimination() {
    // tclsh: `if {1} {set x 1}` runs the body ⇒ unwrap.
    let it = optimised("if {1} {\n    set x 1\n}", TCL);
    assert!(!it.contains("if"));
    assert!(it.contains("set x 1"));

    // `if {0}` deletes the body; trailing statement survives.
    let iff = optimised("if {0} {\n    set x 1\n}\nputs alive", TCL);
    assert!(!iff.contains("set x 1"));
    assert!(iff.contains("puts alive"));

    // `if {0} ... else ...` keeps the else body. tclsh: else runs.
    let ie = optimised("if {0} {\n    set x 1\n} else {\n    set y 2\n}", TCL);
    assert!(!ie.contains("set x 1"));
    assert!(ie.contains("set y 2"));

    // elseif chain selects the first true clause. tclsh: `set b 2` runs.
    let el = optimised(
        "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}",
        TCL,
    );
    assert!(el.contains("set b 2"));
    assert!(!el.contains("set a 1"));
    assert!(!el.contains("set c 3"));

    // All conditions false, no else ⇒ whole thing gone, trailing survives (O112).
    let all_false = "if {0} { set a 1 } elseif {0} { set b 2 }\nputs done";
    assert!(optimised(all_false, TCL).contains("puts done"));
    assert!(opt_fires(all_false, TCL, "O112"));

    // Runtime condition `if {$x}` is untouched.
    assert!(opt_absent("if {$x} {\n    set y 1\n}", TCL, "O112"));
}

#[test]
fn o112_while_for_elimination() {
    // `while {0}` deletes the loop.
    let wf = "while {0} {\n    puts looping\n}\nputs done";
    assert!(!optimised(wf, TCL).contains("puts looping"));
    assert!(optimised(wf, TCL).contains("puts done"));
    assert!(opt_fires(wf, TCL, "O112"));

    // `while {1}` is an infinite loop — NOT eliminated by O112. (The constant `1`
    // condition folds as O101 but the loop is never deleted; O112 does not fire.)
    assert!(opt_absent("while {1} {\n    break\n}", TCL, "O112"));

    // `for {init} {0} ...` keeps only the init.
    let ff = "for {set i 0} {0} {incr i} {\n    puts looping\n}";
    assert!(!optimised(ff, TCL).contains("puts looping"));
    assert!(optimised(ff, TCL).contains("set i 0"));
    assert!(opt_fires(ff, TCL, "O112"));

    // `for {} {0} {} ...` deletes entirely.
    let ffe = "for {} {0} {} {\n    puts looping\n}\nputs done";
    assert!(!optimised(ffe, TCL).contains("puts looping"));
    assert!(optimised(ffe, TCL).contains("puts done"));
    assert!(opt_fires(ffe, TCL, "O112"));
}

#[test]
fn o112_switch_elimination() {
    // tclsh: switch abc {abc {1} def {2}} ⇒ arm 1.
    let lit = optimised(
        "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}",
        TCL,
    );
    assert!(lit.contains("set x 1"));
    assert!(!lit.contains("set y 2"));

    // No literal match ⇒ default. tclsh: switch xyz {...default {2}} ⇒ 2.
    let def = optimised(
        "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}",
        TCL,
    );
    assert!(!def.contains("set a 1"));
    assert!(def.contains("set b 2"));

    // No match, no default ⇒ whole switch deleted.
    let nm = optimised("switch xyz {\n    abc { set a 1 }\n}\nputs done", TCL);
    assert!(!nm.contains("set a 1"));
    assert!(nm.contains("puts done"));

    // Dynamic subject is untouched.
    assert!(opt_absent(
        "switch $x {\n    abc { set a 1 }\n}",
        TCL,
        "O112"
    ));
}

#[test]
fn o112_nested_via_multipass() {
    // First pass unwraps `if {1}`; the fixpoint helper eliminates the inner
    // `if {0}` dead block.
    let nested = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}";
    assert!(optimised(nested, TCL).contains("set alive 2"));
    let registry = static_context_for(TCL).commands();
    let (fixed, _) = optimise_source_multipass(
        nested,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert!(!fixed.contains("set dead 1"));
    assert!(fixed.contains("set alive 2"));
}

// Cross-event DSE guardrails (f5-irules) — stores read in a later event survive

#[test]
fn cross_event_dse_guardrails() {
    // `$uri` set in HTTP_REQUEST, read in HTTP_RESPONSE — the store must survive
    // (connection-scoped variables persist across `when` handlers). (Structural:
    // deleting `set uri` would leave $uri undefined in the response handler.)
    let uri = "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\nwhen HTTP_RESPONSE {\n    log local0. \"uri=$uri\"\n}\n";
    assert!(optimised(uri, IR).contains("set uri"));

    // `[info exists ans_cleared]` in DNS_RESPONSE observes a flag set in
    // DNS_REQUEST — neither the store nor the existence check may be folded.
    let flag = "when DNS_REQUEST {\n    if { [DNS::header opcode] ne \"QUERY\" } {\n        set ans_cleared 1\n        DNS::return\n        return\n    }\n}\nwhen DNS_RESPONSE {\n    if { [info exists ans_cleared] } {\n        unset -nocomplain -- ans_cleared\n        return\n    }\n}\n";
    assert!(optimised(flag, IR).contains("set ans_cleared"));
}
