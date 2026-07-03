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
    // TP / must-alias kill: two writes to the same literal-key element, no intervening read.
    let src = "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}";
    let found = fires(src, D, "W220") || fires(src, D, "O109");
    assert!(
        found,
        "FP-DS-06 TP: two writes to the same array element must surface as W220 or O109; emitted: {:?}",
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
