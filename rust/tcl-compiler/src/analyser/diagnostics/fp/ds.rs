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

//! DS family — dead-store / unused (W220/W211).
//! Pairs to `tests/test_fp_ds.py` and the §DS entries in `docs/design/compiler/FP.md`.

use super::{D, codes, fires};

// ---------------------------------------------------------------------------
// FP-DS-01 — incr/append/lappend inside cmd-sub keeps the init live
// ---------------------------------------------------------------------------

const FP_DS_01_REPRO: &str = "\
proc f {} {
    # incr inside the cmd-sub reads `i` (the prior value) — so the
    # feeding `set i 0` is alive, not a dead store.
    set i 0
    foreach j {1 2 3} { lappend r [incr i $j] }
    return $r
}
";

#[test]
fn fp_ds_01_init_kept_live_by_cmdsub_incr() {
    // FP-DS-01: set i 0 is read-modify-written by [incr i $j]; must NOT fire W220/W211.
    assert!(
        !fires(FP_DS_01_REPRO, D, "W220"),
        "FP-DS-01: set i 0 must NOT fire W220; emitted: {:?}",
        codes(FP_DS_01_REPRO, D)
    );
    assert!(
        !fires(FP_DS_01_REPRO, D, "W211"),
        "FP-DS-01: `i` must NOT fire W211; emitted: {:?}",
        codes(FP_DS_01_REPRO, D)
    );
}

#[test]
fn fp_ds_01_genuine_dead_store_still_fires() {
    // TP control: no read-modify-write of `i`; first assignment is truly dead.
    let src = "proc f {} { set i 0\n set i 5\n return $i }";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-01 TP: real dead store must fire W220; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-02 — reads inside [expr {...}] cmd-sub are real uses
// ---------------------------------------------------------------------------

const FP_DS_02_REPRO: &str = "\
proc f {} {
    # $w is read inside the [expr {...}] cmd-sub — `set w 5` is NOT
    # a dead store, and `w` is NOT unused.
    set w 5
    set i 0
    incr i [expr {$w}]
    return $i
}
";

#[test]
fn fp_ds_02_expr_cmdsub_read_keeps_def_live() {
    // FP-DS-02: [expr {$w}] reads w; set w 5 is alive.
    assert!(
        !fires(FP_DS_02_REPRO, D, "W220"),
        "FP-DS-02: set w 5 must NOT fire W220; emitted: {:?}",
        codes(FP_DS_02_REPRO, D)
    );
    assert!(
        !fires(FP_DS_02_REPRO, D, "W211"),
        "FP-DS-02: `w` must NOT fire W211; emitted: {:?}",
        codes(FP_DS_02_REPRO, D)
    );
}

#[test]
fn fp_ds_02_no_expr_cmdsub_read_still_fires() {
    // TP control: no expr-cmdsub use of `w`; set w 5 IS truly dead.
    let src = "proc f {} { set w 5\n set w 6\n return $w }";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-02 TP: redundant assignment must fire W220; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-03 — eval {literal-body} reads run in caller scope
// ---------------------------------------------------------------------------

const FP_DS_03_REPRO: &str = "\
proc f {} {
    # eval's braced body runs in the current scope; `$x` read here is
    # a real read of the local `x`.
    set x 1
    eval {puts $x}
}
";

#[test]
fn fp_ds_03_eval_body_read_kept_live() {
    // FP-DS-03: eval body reads $x; set x 1 is alive, x is used.
    assert!(
        !fires(FP_DS_03_REPRO, D, "W220"),
        "FP-DS-03: set x 1 must NOT fire W220; emitted: {:?}",
        codes(FP_DS_03_REPRO, D)
    );
    assert!(
        !fires(FP_DS_03_REPRO, D, "W211"),
        "FP-DS-03: `x` must NOT fire W211; emitted: {:?}",
        codes(FP_DS_03_REPRO, D)
    );
}

#[test]
fn fp_ds_03_eval_body_without_read_still_fires() {
    // TP control: eval body doesn't read `x`; x IS truly unused.
    let src = "proc f {} { set x 1\n eval {puts hi} }";
    assert!(
        fires(src, D, "W211"),
        "FP-DS-03 TP: eval body that doesn't read x must fire W211; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-04 — traced variables excluded (soundness)
// ---------------------------------------------------------------------------

const FP_DS_04_REPRO: &str = "\
proc f {} {
    # The write is observable through the callback — must NOT fire
    # W220 (dead-store) or W211 (unused).
    trace add variable x write cb
    set x 1
}
";

#[test]
fn fp_ds_04_traced_var_no_w220() {
    // FP-DS-04: trace-write callback observes every `set x …`; write is not dead.
    assert!(
        !fires(FP_DS_04_REPRO, D, "W220"),
        "FP-DS-04: traced var must NOT fire W220; emitted: {:?}",
        codes(FP_DS_04_REPRO, D)
    );
    assert!(
        !fires(FP_DS_04_REPRO, D, "W211"),
        "FP-DS-04: traced var must NOT fire W211; emitted: {:?}",
        codes(FP_DS_04_REPRO, D)
    );
}

#[test]
fn fp_ds_04_84_form_also_excluded() {
    // FP-DS-04: Tcl 8.4 `trace variable x w cb` form must also be exempted.
    let src = "proc f {} { trace variable x w cb\n set x 1 }";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-04: 8.4-form trace must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W211"),
        "FP-DS-04: 8.4-form trace must NOT fire W211; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_04_untraced_unrelated_var_still_fires() {
    // TP control: tracing `x` must not blanket-suppress an unrelated `y`.
    let src = "proc f {} { trace add variable x write cb\n set y 1 }";
    let w220 = fires(src, D, "W220");
    let w211 = fires(src, D, "W211");
    assert!(
        w220 || w211,
        "FP-DS-04 TP: unrelated `y` must fire W220/W211; emitted: {:?}",
        codes(src, D)
    );
}

// FP-DS-04 cross-scope variant: namespace-global ::w traced in one proc,
// written in another. Fixed by folding module-wide traced ::-globals into every
// function's suppression context (scan_module_traced_globals).
#[test]
fn fp_ds_04_cross_scope_namespace_global_trace() {
    // Must stay silent: the trace on ::w in proc s observes the write `set ::w 1`.
    let src = "trace add variable ::w write h\nproc s {} { set ::w 1 }";
    assert!(
        !fires(src, D, "W211"),
        "FP-DS-04: cross-scope ::w trace must NOT fire W211; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-05 — return value read counts as a use
// ---------------------------------------------------------------------------

const FP_DS_05_REPRO: &str = "\
proc f {} {
    # return $x reads $x — `set x 1` is NOT a dead store, `x` is NOT unused.
    set x 1
    return $x
}
";

#[test]
fn fp_ds_05_return_read_counts() {
    // FP-DS-05: `return $x` is a use of `x`; set x 1 is alive.
    assert!(
        !fires(FP_DS_05_REPRO, D, "W220"),
        "FP-DS-05: set x 1 must NOT fire W220; emitted: {:?}",
        codes(FP_DS_05_REPRO, D)
    );
    assert!(
        !fires(FP_DS_05_REPRO, D, "W211"),
        "FP-DS-05: `x` must NOT fire W211; emitted: {:?}",
        codes(FP_DS_05_REPRO, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-06 — ARRAY_ELEM Place: distinct array elements are distinct stores
// ---------------------------------------------------------------------------

const FP_DS_06_REPRO: &str = "\
proc f {} {
    # k and j are distinct array element Places — set a(k) is NOT
    # killed by set a(j); the read of $a(k) makes the first write live.
    set a(k) 1
    set a(j) 2
    puts $a(k)
}
";

#[test]
fn fp_ds_06_array_elem_writes_distinct() {
    // FP-DS-06: distinct array-element writes do not kill each other.
    assert!(
        !fires(FP_DS_06_REPRO, D, "W220"),
        "FP-DS-06: set a(k) 1 must NOT fire W220 — set a(j) 2 writes a different Place; emitted: {:?}",
        codes(FP_DS_06_REPRO, D)
    );
}

#[test]
fn fp_ds_06_same_element_overwrite_fires_w220() {
    // TP / must-alias kill: two writes to the same literal-key element, no
    // intervening read.  tclsh 8.6: `proc f {} {set a(k) 1; set a(k) 2;
    // return $a(k)}; f` → 2, and with a write trace attached the callback
    // fires for BOTH writes — so without a trace/alias the first write is
    // genuinely unobservable, a true dead store.
    let src = "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}";
    let found = fires(src, D, "W220") || fires(src, D, "O109");
    assert!(
        found,
        "FP-DS-06 TP: two writes to the same array element must surface as W220 or O109; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_06_cmdsub_read_between_same_element_writes_cancels_kill() {
    // Adjacent FP guard for the must-alias kill: a read of `a(k)` buried in a
    // command substitution BETWEEN the two writes observes the first write,
    // so the kill is cancelled and nothing may fire.  tclsh 8.6: `n` is 1
    // (the length of "1") — deleting the first write would change it.
    let src = "proc f {} {\n    set a(k) 1\n    set n [string length $a(k)]\n    set a(k) 2\n    return \"$a(k) $n\"\n}";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-06: a cmd-sub read between same-element writes must cancel the kill; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W211"),
        "FP-DS-06: `a` is read; must NOT fire W211; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_06_traced_array_same_element_overwrite_silent() {
    // Adjacent FP guard: with a write trace on the array, the callback
    // observes BOTH writes (tclsh: the trace fires twice), so the first
    // same-key write is not a dead store.
    let src = "proc f {} {\n    trace add variable a write cb\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-06: a traced array's same-element overwrite must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-07 — namespace-eval body scope survives IRBlock rebuild
// ---------------------------------------------------------------------------

const FP_DS_07_REPRO: &str = "\
proc reset {} { uplevel 1 {set counter 0} }
proc g {x} {
    namespace eval ::ns {
        reset
        puts \"hello $x\"
    }
}
";

#[test]
fn fp_ds_07_ns_eval_param_unused_through_rebuild_fires() {
    // TP: $x in ns-eval body runs in ::ns (not g's frame); param `x` is genuinely unused.
    // FP-DS-07: W214 must fire after IRBlock rebuild preserves caller_scope=False.
    assert!(
        fires(FP_DS_07_REPRO, D, "W214"),
        "FP-DS-07 TP: param used only inside ns-eval body must fire W214; emitted: {:?}",
        codes(FP_DS_07_REPRO, D)
    );
}

#[test]
fn fp_ds_07_plain_eval_body_read_is_caller_use_silent() {
    // FP control: plain eval body runs in caller frame; $x IS a use of the param.
    let src = "proc g {x} { eval { puts \"hello $x\" } }";
    assert!(
        !fires(src, D, "W214"),
        "FP-DS-07: plain eval body read must NOT fire W214; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-08 — dict with key-aware suppression on the return-terminator path
// ---------------------------------------------------------------------------

const FP_DS_08_REPRO: &str = "proc f {} { set d {}; dict with d {}; return $missing }\n";

#[test]
fn fp_ds_08_empty_dict_with_return_missing_fires() {
    // TP: empty literal dict; dict with unpacks no keys; return $missing fires W210.
    // FP-DS-08: CFGReturn arm must apply key-aware logic (D4-F3 closure).
    assert!(
        fires(FP_DS_08_REPRO, D, "W210"),
        "FP-DS-08 TP: empty dict with does not unpack 'missing'; return $missing must fire W210; emitted: {:?}",
        codes(FP_DS_08_REPRO, D)
    );
}

#[test]
fn fp_ds_08_known_key_dict_with_return_var_silent() {
    // TN control: literal dict has `missing` as key; dict with unpacks it; reading $missing is safe.
    let src = "proc f {} { set d {missing ok}; dict with d {}; return $missing }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-DS-08: known-key dict with must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_08_unknown_dict_with_return_var_silent() {
    // TN control: dict shape unknown (callee param); must conservatively stay silent.
    let src = "proc f {d} { dict with d {}; return $missing }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-DS-08: unknown-shape dict with must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-09 — interproc literal-dict propagation feeds the dict-with key check
// ---------------------------------------------------------------------------

const FP_DS_09_REPRO: &str = "proc f {d} { dict with d { return $missing } }\nf {}\n";

#[test]
fn fp_ds_09_interproc_empty_dict_fires() {
    // TP: caller passes literal {}; interproc propagation makes callee see d#0=CONST('').
    // FP-DS-09: dict-with key check exempts no names; return $missing fires W210.
    assert!(
        fires(FP_DS_09_REPRO, D, "W210"),
        "FP-DS-09 TP: caller-passed empty dict must propagate to callee dict-with key check; emitted: {:?}",
        codes(FP_DS_09_REPRO, D)
    );
}

#[test]
fn fp_ds_09_interproc_key_present_silent() {
    // TN control: caller passes {missing ok}; dict with unpacks it; return $missing is safe.
    let src = "proc f {d} { dict with d { return $missing } }\nf {missing ok}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-DS-09: key-present interproc dict must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_09_interproc_mixed_callers_conservative_silent() {
    // TN control: callers disagree on literal; collector falls back conservative; W210 must NOT fire.
    let src = "proc f {d} { dict with d { return $missing } }\nf {}\nf {missing X}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-DS-09: mixed callers must NOT fire W210 (conservative); emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-10 — reads nested inside a `dict for`/`dict map` body keep the store
//            live (issue #833)
//
// `dict for`/`dict map` run their body in the caller's frame, so the analysis
// CFG now flattens the body into real loop blocks (like `foreach`) instead of
// re-emitting it as an opaque barrier whose reads were recovered by a shallow
// word scan.  That shallow scan only saw top-level `$var` / `[...]` tokens, so a
// read one brace level deep — e.g. `$x` used as the command name of `$x a $key`
// inside an `if` — was invisible, and the feeding `set x set` looked like a dead
// store.  The fix makes such reads first-class SSA uses.
// ---------------------------------------------------------------------------

// The exact issue #833 reproducer: `$x` is read as the command name of the
// dispatched call, nested inside `if {$value}` inside `dict for`.
const FP_DS_10_REPRO: &str = "\
proc demo {} {
    set x set
    set d [dict create a true b false c true]
    dict for {key value} $d {
        if {$value} {
            $x a $key
        }
    }
}
";

#[test]
fn fp_ds_10_command_name_read_in_dict_for_if_keeps_store_live() {
    // FP-DS-10: the command-name read of `x`, nested inside `if` inside
    // `dict for`, must keep `set x set` alive — no W220 and no W211.
    assert!(
        !fires(FP_DS_10_REPRO, D, "W220"),
        "FP-DS-10: nested command-name read must NOT fire W220; emitted: {:?}",
        codes(FP_DS_10_REPRO, D)
    );
    assert!(
        !fires(FP_DS_10_REPRO, D, "W211"),
        "FP-DS-10: `x` is read, must NOT fire W211; emitted: {:?}",
        codes(FP_DS_10_REPRO, D)
    );
}

#[test]
fn fp_ds_10_dict_map_variant_keeps_store_live() {
    // FP-DS-10: `dict map` shares the same body-recovery path as `dict for`.
    let src = "\
proc demo {} {
    set x set
    set d [dict create a 1]
    dict map {key value} $d { if {$value} { $x a $key } }
}
";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-10: dict map nested read must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
    assert!(!fires(src, D, "W211"), "emitted: {:?}", codes(src, D));
}

#[test]
fn fp_ds_10_deeply_nested_read_keeps_store_live() {
    // FP-DS-10: two brace levels of `if` deep inside `dict for` — the read must
    // still reach the store through the body's own lowered control flow.
    let src = "\
proc demo {} {
    set x set
    set d [dict create a 1]
    dict for {k v} $d {
        if {$v} {
            if {$k ne \"\"} {
                $x a $k
            }
        }
    }
}
";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-10: doubly-nested read must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_10_plain_var_read_in_dict_for_if_keeps_store_live() {
    // FP-DS-10: not just command-name reads — an ordinary `$msg` argument read
    // nested inside `if` inside `dict for` must keep its store live too.
    let src = "\
proc demo {} {
    set msg hello
    set d [dict create a 1]
    dict for {k v} $d {
        if {$v} {
            puts $msg
        }
    }
}
";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-10: nested `$msg` read must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
    assert!(!fires(src, D, "W211"), "emitted: {:?}", codes(src, D));
}

#[test]
fn fp_ds_10_real_dead_store_before_dict_for_still_fires() {
    // TP control: `set x set` is overwritten by `set x puts` before any read, so
    // it is a genuine dead store even though a `dict for` uses the live version.
    // Flattening the body must not mask a real dead store outside it.
    let src = "\
proc demo {d} {
    set x set
    set x puts
    dict for {k v} $d { $x $k }
}
";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-10 TP: real dead store must still fire W220; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_10_dead_store_inside_dict_for_body_now_fires() {
    // TP control (precision gained by the fix): a dead store *inside* the body
    // was invisible while the body was an opaque barrier; now that the body is
    // lowered it is a real W220.
    let src = "\
proc demo {d} {
    dict for {k v} $d {
        set tmp 1
        set tmp 2
        puts $tmp
    }
}
";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-10 TP: dead store inside a dict-for body must fire W220; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_10_write_only_local_in_dict_for_body_still_flags() {
    // FN guard: a local written but never read inside the body must still be
    // reported — the fix must not manufacture a phantom read that hides it.
    let src = "\
proc demo {d} {
    dict for {k v} $d {
        set unusedvar 1
        puts $k
    }
}
";
    assert!(
        fires(src, D, "W220") || fires(src, D, "W211"),
        "FP-DS-10 FN guard: write-only body local must still flag; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_10_clean_dict_for_is_silent() {
    // TN control: a clean `dict for` that only reads its loop vars is silent.
    let src = "proc demo {d} { dict for {k v} $d { puts \"$k=$v\" } }";
    assert!(
        codes(src, D).iter().all(|c| c != "W220" && c != "W211"),
        "FP-DS-10 TN: clean dict for must not fire W220/W211; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-11 — an `uplevel` body runs in another stack frame; its variable
//            references belong to that frame, not the enclosing proc
//            (issue #837)
//
// `uplevel ?level? {body}` now carries an `ArgRole::Body` on its script word
// (a registry-driven `arg_role_resolver`), so the body is recursed and
// analysed like every other script body instead of being an opaque string.
// The spec is also `BodyKind::Structural`: the body evaluates in the frame
// named by `level`, so a `$var` read inside a *braced* body resolves against
// that frame — it does NOT count as a use of an enclosing-proc local of the
// same name.  A value substituted at the enclosing level (`[list …]`, a
// quoted body, or a plain local read) is evaluated in the enclosing frame and
// keeps the local live, exactly as before.
// ---------------------------------------------------------------------------

#[test]
fn fp_ds_11_caller_frame_write_is_not_an_enclosing_dead_store() {
    // FP/TN: `set counter 0` writes the *caller's* `counter`; the enclosing
    // proc has no `counter` local, so nothing is a dead store here.
    let src = "proc f {} { uplevel 1 {set counter 0} }";
    assert!(
        !fires(src, D, "W220") && !fires(src, D, "W211"),
        "FP-DS-11: a caller-frame write must not flag an enclosing dead store; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_11_list_substituted_read_keeps_enclosing_store_live() {
    // FP: `[list puts $x]` is built in the enclosing frame — `$x` reads the
    // enclosing `x`, so `set x 1` is live.  Structural must NOT skip a
    // command-substitution body (only a braced one shifts frame).
    let src = "proc f {} { set x 1\n uplevel 1 [list puts $x] }";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-11: a `[list]`-substituted read keeps the enclosing store live; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_11_local_read_keeps_store_live_alongside_uplevel() {
    // FP: `x` is read locally before the uplevel, so it is unambiguously
    // live regardless of the frame-shifted body's own `$x`.
    let src = "proc f {} { set x 1\n puts $x\n uplevel 1 {puts $x} }";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-11: a local read keeps the store live; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_11_braced_body_only_read_is_a_frame_shifted_dead_store() {
    // TP (frame-aware precision): `$x` appears *only* inside the braced
    // `uplevel 1 {…}` body, which reads the caller's frame — so the enclosing
    // `set x 1` is genuinely never read in `f` and is a real dead store.
    let src = "proc f {} { set x 1\n uplevel 1 {puts $x} }";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-11 TP: an enclosing store read only in a braced uplevel body is dead; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_11_real_dead_store_outside_uplevel_still_fires() {
    // TP: an ordinary dead store in the enclosing proc, next to an uplevel,
    // still fires — recursing the body has not masked enclosing analysis.
    // `dead` is overwritten before its `return` read, so the first `set` is a
    // genuine dead store (W220) — and because `dead` is used, there is no
    // co-located W211 to dedup it against.
    let src = "proc f {} { set dead 1\n set dead 2\n uplevel 1 {set counter 0}\n return $dead }";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-11 TP: a real dead store beside an uplevel still fires; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_11_clean_uplevel_body_is_silent() {
    // TN control: a clean braced body that only uses its own loop var is
    // silent — the body is analysed, not flagged.
    let src = "proc f {} { uplevel 1 {foreach x $l { puts $x }} }";
    assert!(
        codes(src, D)
            .iter()
            .all(|c| c != "W220" && c != "W211" && c != "W210"),
        "FP-DS-11 TN: a clean uplevel body must be silent; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-12 — a dynamic-name read makes every store observable
// (issue #923 audit idx 2 and 64)
// ---------------------------------------------------------------------------
//
// `foreach v [info locals] { … [set $v] … }` reads every local through a name
// no literal `$x` token spells, so neither "set but never used" (W211) nor
// "assignment never read" (W220) is provable once such a read exists.
//
// Oracle (tclsh 9.0.4 and 8.6.14, identical):
//   proc collect {} { set alpha 10; set beta 20; set gamma 30
//                     set r [dict create]
//                     foreach v [info locals] {
//                         if {$v in {r v}} { continue }
//                         dict append r $v [subst $[subst $v]] }
//                     return $r }
//   collect                                  → alpha 10 beta 20 gamma 30
//   set ns "ctx"; puts "{ $ns"               → `{ ctx`

const FP_DS_12_REPRO: &str = "\
proc collect {} {
    set alpha 10
    set beta 20
    set resultDict [dict create]
    foreach locVar [info locals] {
        dict append resultDict $locVar [subst $[subst $locVar]]
    }
    return $resultDict
}
";

#[test]
fn fp_ds_12_double_subst_dereference_keeps_locals_live() {
    assert!(
        !fires(FP_DS_12_REPRO, D, "W211"),
        "FP-DS-12: `[subst $[subst $locVar]]` reads every local; emitted: {:?}",
        codes(FP_DS_12_REPRO, D)
    );
    assert!(
        !fires(FP_DS_12_REPRO, D, "W220"),
        "FP-DS-12: no store is provably dead beside a dynamic read; emitted: {:?}",
        codes(FP_DS_12_REPRO, D)
    );
}

#[test]
fn fp_ds_12_single_indirect_dereference_keeps_locals_live() {
    // The audit's second control: one level of indirection (`[set $locVar]`)
    // reproduces the same false positives.
    let src = "\
proc collect2 {} {
    set alpha 10
    set beta 20
    set out {}
    foreach locVar [info locals] {
        lappend out $locVar [set $locVar]
    }
    return $out
}
";
    assert!(
        !fires(src, D, "W211"),
        "FP-DS-12: `[set $locVar]` reads every local; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_12_dynamic_read_keeps_an_overwritten_store_live() {
    let src = "proc f {n} { set a 1\n puts [set $n]\n set a 2\n return $a }\n";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-12: the dynamic read may observe the first store; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_12_genuine_unused_and_dead_store_still_fire() {
    // TP controls: with no dynamic read in the function, both verdicts stand.
    let unused = "proc g {} { set alpha 10; set beta 20; return $beta }\n";
    assert!(
        fires(unused, D, "W211"),
        "FP-DS-12 TP: a genuinely unused local must still fire W211; emitted: {:?}",
        codes(unused, D)
    );
    let dead = "proc f {} { set a 1\n set a 2\n return $a }\n";
    assert!(
        fires(dead, D, "W220"),
        "FP-DS-12 TP: a genuine dead store must still fire W220; emitted: {:?}",
        codes(dead, D)
    );
}

#[test]
fn fp_ds_12_literal_subst_template_is_not_a_dynamic_read() {
    // TN control: `subst {$a}` carries its names in source text, so the
    // ordinary scanners see them and the barrier stays clear — an unrelated
    // dead store in the same proc must still fire.
    let src = "proc f {} { set a 1\n set b 1\n set b 2\n return [subst {$a$b}] }\n";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-12 TN: a literal subst template must not blind the pass; \
emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-13 — an unmatched `{` inside a double-quoted string does not hide
// the `$var` read (issue #923 audit idx 64, `namespaces` corpus `pix`)
// ---------------------------------------------------------------------------
//
// The dead-store read scan runs over the segmenter's already-dequoted word
// text.  Re-lexing that text with ordinary top-level rules used to read a
// stray `{` as opening a brace-quoted (non-substituting) word, so `$ns`
// stopped being a read and every `set ns …` feeding a multi-line
// `puts $fp "namespace eval ::pix {\n … $ns …\n}"` looked dead.
//
// Oracle (tclsh 9.0.4 and 8.6.14, identical):
//   set ns "ctx"; puts "{ $ns"               → `{ ctx`

#[test]
fn fp_ds_13_unmatched_brace_in_quoted_string_keeps_the_read() {
    let src = "set ns \"ctx\"\nputs \"{ $ns\"\n";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-13: `$ns` inside `\"{{ $ns\"` is a read; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_13_real_pixdoc_generation_shape_keeps_the_reads() {
    let src = "\
proc gen {fp} {
    set ns \"ctx\"
    set preamble \"hello\"
    puts $fp \"namespace eval ::pix {
        namespace eval $ns {
            variable _ruff_preamble $preamble
        }
    }\"
}
";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-13: both `set ns` and `set preamble` feed the string; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W211"),
        "FP-DS-13: neither local is unused; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_ds_13_overwritten_store_beside_a_quoted_read_still_fires() {
    // TP control: the same shape with a genuinely overwritten first store.
    let src = "proc f {} { set ns \"ctx\"\n set ns \"other\"\n return \"{ $ns\" }\n";
    assert!(
        fires(src, D, "W220"),
        "FP-DS-13 TP: a real dead store beside a quoted read still fires; \
emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-DS-14 — a brace-quoted variable name does not blind the function
// (PR #1076 review, P2)
// ---------------------------------------------------------------------------
//
// `{$n}` is Tcl's literal spelling for a variable *called* `$n`; nothing is
// computed, so the dynamic-name barrier must stay clear and every diagnostic
// in the function must stay live.
//
// Oracle (tclsh 9.0.4 and 8.6.14, identical):
//   set {$n} v; info exists {$n} → 1 ; info exists n → 0
//   set i 5; set {arr($i)} 1; array names arr → {$i} ; info exists arr(5) → 0

#[test]
fn fp_ds_14_brace_quoted_name_keeps_unrelated_diagnostics_live() {
    for (label, src) in [
        (
            "write",
            "proc f {} { set {$n} 1\n set a 1\n set a 2\n return $a }\n",
        ),
        (
            "read",
            "proc f {} { set b [set {$n}]\n set a 1\n set a 2\n return \"$a$b\" }\n",
        ),
        (
            "destroy",
            "proc f {} { unset -nocomplain {$n}\n set a 1\n set a 2\n return $a }\n",
        ),
    ] {
        assert!(
            fires(src, D, "W220"),
            "FP-DS-14 ({label}): a brace-quoted name is literal and must not \
blind the dead-store pass; emitted: {:?}",
            codes(src, D)
        );
    }
}

#[test]
fn fp_ds_14_unbraced_computed_name_still_blinds_the_pass() {
    // TN control: dropping the braces makes the name genuinely computed, so
    // the read barrier applies and W220 correctly goes silent.
    let src = "proc f {n} { set b [set $n]\n set a 1\n set a 2\n return \"$a$b\" }\n";
    assert!(
        !fires(src, D, "W220"),
        "FP-DS-14 TN: `[set $n]` may observe the first store; emitted: {:?}",
        codes(src, D)
    );
}
