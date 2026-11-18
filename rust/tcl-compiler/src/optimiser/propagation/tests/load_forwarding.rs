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

//! O102: a single reaching literal forwarded to its uses, name arguments, globals a callee reassigns and hidden writes.

use super::*;

#[test]
fn load_forwarding_fires_o102_for_single_reaching_def() {
    // `set n 7; puts $n` — single reaching def is literal 7
    // → emit O102 on the puts use site.
    let opts = run_pass("set n 7\nputs $n");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O102),
        "expected O102 load-forwarding, got {opts:?}",
    );
}

/// A definition whose value SCCP proved still forwards.
///
/// `set a 1; incr a` leaves `a` provably `2`, and that is what a use of it
/// should see. O102 only ever recognised a *syntactic* literal as a
/// reaching definition, so the chain stopped at the `incr` and the whole
/// snippet optimised to nothing. The name-keyed `sccp_constants_for`
/// projection cannot rescue it either: `a` holds `1` at one version and
/// `2` at another, so that map drops the variable entirely.
#[test]
fn a_computed_definition_forwards_the_constant_sccp_proved() {
    for (source, want) in [
        ("set a 1\nincr a\nputs \"$a\"\n", "2"),
        ("set a 1\nincr a\nputs $a\n", "2"),
        ("set a 1\nincr a 5\nputs $a\n", "6"),
    ] {
        let opts = run_pass(source);
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O100 && !o.hint_only && o.replacement == want),
            "expected an applicable O100 inlining {want:?} for {source:?}, got {opts:?}",
        );
    }
}

/// A hint-only forward records no replacement.
///
/// Its span is the whole consuming statement, so the literal is not a
/// valid replacement for it — recording one anyway is what let a hint on
/// `incr a` read as a one-click rewrite to a bare `1`.
#[test]
fn a_hint_only_forward_carries_no_replacement() {
    let opts = run_pass("set n 7\nputs \"n=$n\"\n");
    let hints: Vec<&Optimisation> = opts.iter().filter(|o| o.hint_only).collect();
    assert!(!hints.is_empty(), "expected a hint-only O102, got {opts:?}");
    assert!(
        hints.iter().all(|o| o.replacement.is_empty()),
        "a hint with no sub-span target must carry no payload, got {hints:?}",
    );
}

/// A variable-*name* argument is not an operand, so no propagation code
/// may target it.
///
/// `set a 1; incr a` has one reaching literal for `a`, and `incr a`'s only
/// mention of `a` names the cell it mutates. Forwarding the literal there
/// produced `O102 … → 1` over the whole of `incr a`: a rewrite that
/// destroys the increment and leaves `1` in command position, where tclsh
/// answers `invalid command name "1"`. It was unreachable behind the
/// `hint_only` guard, but it was a wrong payload sitting one guard away
/// from being applied, and wrong advice about the statement either way.
#[test]
fn no_o1xx_targets_a_variable_name_argument() {
    // One row per registry role that names a variable rather than passing
    // a value: `VarWrite` read-modify-write, `VarRead`, and the
    // destroying form. Each is the *only* mention of the variable in its
    // statement, so any O1xx on that line is targeting the name.
    for source in [
        "set a 1\nincr a\n",
        "set a 1\nappend a x\n",
        "set a 1\nlappend a x\n",
        "set a 1\ndict incr a k\n",
        "set a 1\ninfo exists a\n",
        "set a 1\nunset a\n",
        "set a 1\nbinary scan xy c a\n",
        "set a 1\nregexp {(x)} xy -> a\n",
    ] {
        let opts = run_pass(source);
        let forwarded: Vec<&Optimisation> = opts
            .iter()
            .filter(|o| o.code.as_str().starts_with("O1") && o.replacement == "1")
            .collect();
        assert_eq!(
            forwarded.len(),
            0,
            "no O1xx may forward the literal into a variable-name position \
                 of {source:?}, got {forwarded:?}",
        );
    }
}

/// The guard is about the *position*, not the command: the same commands
/// still forward into a genuine operand word beside the name.
#[test]
fn a_value_operand_beside_a_name_argument_still_forwards() {
    let opts = run_pass("set n 7\nset a {}\nlappend a $n\n");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O102),
        "the `$n` operand is still an operand, got {opts:?}",
    );
}

// A computed variable name (`set $name 2`) between the
// "sole" reaching def and the use can rewrite `x` under a spelling the
// def-use chains cannot see: `f x` prints 2 in tclsh, so forwarding the
// literal 1 (via O102's chain scan or O100's SCCP projection) is a
// miscompile. The whole function abstains.
#[test]
fn dynamic_name_write_blocks_load_forwarding() {
    let opts = run_pass("proc ::f {name} { set x 1; set $name 2; puts $x }");
    assert!(
        opts.iter()
            .all(|o| !matches!(o.code, DiagCode::O100 | DiagCode::O102)),
        "must not forward the pre-dynamic-write literal 1, got {opts:?}",
    );
}

/// The native-stack depth cap is not evidence that no
/// dynamic write exists below it. Keep the actual O100/O102 consumer in
/// this regression: the source mutation is hidden past the cap, yet it
/// must still prevent forwarding the stale `x = 1` into the return.
#[test]
fn depth_exhaustion_blocks_dynamic_name_load_forwarding() {
    let mut nested = "[set $name 2]".to_owned();
    for _ in 0..=crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.0 {
        nested = format!("[list {nested}]");
    }
    let source = format!("proc ::f {{name}} {{ set x 1; set sink {nested}; return $x }}");
    let reg = registry();
    let cu = CompilationUnit::build_for(&source, &reg, false);
    assert_eq!(
        cu.function("::f")
            .expect("procedure analysed")
            .dynamic_names,
        crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT,
        "the depth cap must fail closed before the optimiser runs"
    );
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    ctx.registry = Some(&reg);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    let opts = ctx.optimisations;
    assert!(
        opts.iter().all(|opt| {
            !matches!(opt.code, DiagCode::O100 | DiagCode::O102) || opt.replacement != "return 1"
        }),
        "a hidden dynamic write must block the stale return fold: {opts:?}"
    );
}

// Regression: a top-level `global` name reassigned by a proc call must
// never be folded as if its "sole reaching def" were stable — on tclsh
// 8.6/9.0, `set tcl_precision 4; proc helper {} {global tcl_precision;
// set tcl_precision 17}; helper; puts $tcl_precision` prints `17`, not
// the `4` a sole-reaching-def fold would give.  See
// `crate::var_observability::scan_module_global_names`.
#[test]
fn o102_does_not_forward_top_level_global_reassigned_by_callee() {
    let src = "set g 4\nproc helper {} { global g\nset g 17 }\nhelper\nputs $g";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "4")),
        "must not fold the stale pre-call literal 4, got {opts:?}",
    );
}

// TN control: an ordinary proc that itself `global`-declares the name it
// reassigns is *already* protected via SCCP/O100's per-function escaping
// set — O102's independent def-use-chain path must apply the identical
// guard rather than bypassing it (this fired incorrectly before O102
// consulted `var_observability` at all).
#[test]
fn o102_does_not_forward_proc_local_global_reassigned_by_callee() {
    let src = "proc p {} {\n    global g\n    set g 4\n    helper\n    puts $g\n}\nproc helper {} { global g\nset g 17 }\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "4")),
        "must not fold the stale pre-call literal 4, got {opts:?}",
    );
}

// Precision control: a top-level name that *no* procedure ever
// `global`-declares must still fold — the whole-module scan must not
// over-widen to every top-level variable.
//
// Note this now folds via O100, not O102: the intervening call to
// `other` is a command O102 cannot prove pure (it has no interprocedural
// purity or upvar-reach information — a callee can dynamically alias
// any name in the caller's frame via `upvar` with no textual
// `global`/`variable` declaration anywhere), so O102's own independent
// same-block scan conservatively withholds. SCCP (O100) already proves
// `safe_const` a genuine whole-function constant via its own escaping
// set (which correctly found no alias/trace anywhere for it), so the
// fold still happens end to end.
#[test]
fn o102_still_forwards_top_level_global_no_proc_touches() {
    let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\nputs $safe_const";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .any(|o| matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "42"),
        "expected an untouched top-level constant to still fold, got {opts:?}",
    );
}

// TN control: a plain proc-local variable (never `global`/`variable`
// declared) is genuinely private to that call frame — O102 must still
// forward it.
#[test]
fn o102_still_forwards_genuinely_local_proc_variable() {
    let src = "proc p {} {\n    set n 7\n    puts $n\n}\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O102 && o.replacement == "7"),
        "expected O102 to still fold a genuinely private proc-local var, got {opts:?}",
    );
}

// Regression: a literal-body `interp eval {}` (targeting the *current*
// interpreter — Tcl's documented meaning of the empty-string path) can
// reassign a variable in the calling scope exactly like an opaque call,
// even though it carries no `global`/`variable`/`upvar`/`trace`
// declaration for it. Confirmed against tclsh 8.6/9.0: `set n 5; interp
// eval {} {set n 99}; puts $n` prints `99`, not the stale `5`.
#[test]
fn o102_does_not_forward_past_interp_eval_barrier() {
    let src = "set n 5\ninterp eval {} { set n 99 }\nputs $n\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O102 && o.replacement == "5")),
        "must not forward the stale literal 5 past `interp eval {{}}`, got {opts:?}",
    );
}

// Regression: a literal-body `uplevel #0 {…}` evaluates in the absolute
// global frame — at top level that coincides with the calling scope, so
// it can reassign a variable there too, with no alias declaration of its
// own. Confirmed against tclsh 8.6/9.0: `set n 5; uplevel #0 {set n
// 99}; puts $n` prints `99`.
#[test]
fn o102_does_not_forward_past_uplevel_hash0_upframe() {
    let src = "set n 5\nuplevel #0 { set n 99 }\nputs $n\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O102 && o.replacement == "5")),
        "must not forward the stale literal 5 past `uplevel #0`, got {opts:?}",
    );
}

#[test]
fn o102_operand_span_fires_applicable_on_call_argv() {
    // `set n 7; puts $n` — the O102 emitted on the puts use
    // must target the `$n` argv span (not the whole Call) and
    // be applicable (not hint-only). The argv span lands on
    // the last 2 chars of the source: `$n`.
    let source = "set n 7\nputs $n";
    let opts = run_pass(source);
    let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
    assert!(
        !o102s.is_empty(),
        "expected at least one O102, got {opts:?}"
    );
    let found = o102s.iter().any(|o| {
        let start = o.span.start() as usize;
        let end = o.span.end() as usize;
        !o.hint_only && o.replacement == "7" && end <= source.len() && &source[start..end] == "$n"
    });
    assert!(
        found,
        "expected applicable O102 with argv span covering `$n`, got {o102s:?}"
    );
}

#[test]
fn o102_operand_span_matches_braced_var_word() {
    // ${n} forms must also be recognised at the argv-text
    // level and produce an applicable rewrite.
    let source = "set n 7\nputs ${n}";
    let opts = run_pass(source);
    let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
    assert!(
        o102s.iter().any(|o| !o.hint_only
            && o.replacement == "7"
            && &source[o.span.start() as usize..o.span.end() as usize] == "${n}"),
        "expected applicable O102 over ${{n}}, got {o102s:?}",
    );
}

#[test]
fn o102_falls_back_to_hint_only_when_var_is_inside_interpolation() {
    // `$n` appears only inside a double-quoted composite word
    // — no argv text matches the bare `$n` form, so the
    // rewrite stays hint-only (the O100 string-interpolation
    // path handles the actual inlining).
    let source = "set n 7\nputs \"n=$n\"";
    let opts = run_pass(source);
    let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
    assert!(!o102s.is_empty(), "expected O102 hint, got {opts:?}");
    assert!(
        o102s.iter().all(|o| o.hint_only),
        "expected all O102s to be hint-only for interpolated use, got {o102s:?}",
    );
}

/// A literal a write the statements do not show may have replaced is
/// never forwarded: an arm of a `switch` the flow graph keeps as one
/// statement, a callback script, or a command the module cannot see.
#[test]
fn o100_and_o102_never_forward_a_literal_a_hidden_write_may_have_replaced() {
    for (src, stale) in [
        (
            "set go 1\nset s q1\nswitch -glob -- $s { q* { set go 0 } }\nputs $go",
            "1",
        ),
        (
            "set go 1\nswitch -nocase -- [gets stdin] { q { set go 0 } }\nputs $go",
            "1",
        ),
        (
            "proc p {s} {\n set go 1\n switch -glob -- $s { q* { set go 0 } }\n puts $go\n}",
            "1",
        ),
        (
            "set go 1\ntrace add variable x write { set ::go 0 ;# }\nset x 1\nputs $go",
            "1",
        ),
        (
            "set done 0\nafter 10 { set done 1 }\nafter 50\nupdate\nputs $done",
            "0",
        ),
        (
            "proc tick {} { set ::done 1 }\nset done 0\nafter 10 tick\nafter 50\nupdate\nputs $done",
            "0",
        ),
        // A callback built by a substitution names no variable; tclsh
        // 8.4-9.1 print 1.
        (
            "proc tick {n} { set ::done $n }\nset done 0\nafter 10 [list tick 1]\nafter 50\nupdate\nputs $done",
            "0",
        ),
        ("set g 5\nfoo\nputs $g", "5"),
        ("set ::g 5\nfoo\nputs $::g", "5"),
        ("proc p {} {\n set g 5\n foo\n puts $g\n}", "5"),
        ("set g 5\nsource other.tcl\nputs $g", "5"),
        ("proc p {} {\n set g 5\n source other.tcl\n puts $g\n}", "5"),
        (
            "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n set line 5\n switch -glob -- $s { g* { zero line } }\n puts $line\n}",
            "5",
        ),
        (
            "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\nputs $go",
            "1",
        ),
    ] {
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102)
                    && o.replacement == stale)),
            "{src}: {opts:?}"
        );
    }
}

/// The controls: a name the hidden writes never name, a name assigned
/// after the call the module cannot see, and a read the call follows still
/// forward.
#[test]
fn o100_and_o102_still_forward_a_literal_no_hidden_write_can_replace() {
    for (src, literal) in [
        ("proc p {} {\n set g 5\n puts $g\n foo\n}", "5"),
        (
            "set go 1\nswitch -glob -- [gets stdin] { q* { set other 0 } }\nputs $go",
            "1",
        ),
        ("foo\nset g 5\nputs $g", "5"),
        (
            "proc hit {} {global hits; incr hits}\nproc p {s} {\n set n 5\n switch -glob -- $s { g* { hit } }\n puts $n\n}",
            "5",
        ),
    ] {
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .any(|o| matches!(o.code, DiagCode::O100 | DiagCode::O102)
                    && o.replacement == literal),
            "{src}: {opts:?}"
        );
    }
}
