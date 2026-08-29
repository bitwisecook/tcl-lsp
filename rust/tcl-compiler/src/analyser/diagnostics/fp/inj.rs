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

//! INJ family — injection / style (W301 uplevel, W101 eval-injection, T102
//! option-injection taint).
//!
//! Most T102 cases here are iRules-dialect (`HTTP::uri` / `HTTP::path` taint
//! sources), so they run under the `f5-irules` dialect rather than the
//! default `D` — except FP-INJ-07, whose `exec` sink is excluded from the
//! iRules dialect and so runs under `D` with a generic (`gets`) source.

use super::{D, codes, fires};

const IRULES: &str = "f5-irules";

// FP-INJ-01 — `uplevel 1 $body` is the safe canonical idiom.

const FP_INJ_01_REPRO: &str = "\
proc f {body} {
    # The canonical `uplevel 1 $body` pattern — must NOT fire W301.
    uplevel 1 $body
}
";

#[test]
fn fp_inj_01_bare_var_no_w301() {
    // FP-INJ-01: single pure-var `uplevel 1 $body` is safe.
    assert!(
        !fires(FP_INJ_01_REPRO, D, "W301"),
        "FP-INJ-01: bare-var uplevel must NOT fire W301; emitted {:?}",
        codes(FP_INJ_01_REPRO, D)
    );
}

#[test]
fn fp_inj_01_quoted_interpolation_still_w301() {
    // TP control: a quoted-interpolated `uplevel "$cmd $arg"` IS unsafe; either
    // W301 or W105 (eval-style injection) may claim it.
    let src = "proc f {cmd arg} { uplevel 1 \"$cmd $arg\" }";
    assert!(
        fires(src, D, "W301") || fires(src, D, "W105"),
        "FP-INJ-01 TP: quoted-interpolation uplevel must warn (W301/W105); emitted {:?}",
        codes(src, D)
    );
}

// FP-INJ-02 — `eval [list ...]` is the canonical safe form.

#[test]
fn fp_inj_02_eval_list_clean() {
    // FP-INJ-02: `eval [list set $var $val]` is canonical-safe.
    let src = "eval [list set $varname $value]";
    assert!(
        !fires(src, D, "W101"),
        "FP-INJ-02: eval [list ...] must NOT fire W101; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_inj_02_eval_linsert_clean() {
    // FP-INJ-02: any list-returning canonical command is exempt.
    let src = "eval [linsert $cmdlist 0 extraarg]";
    assert!(
        !fires(src, D, "W101"),
        "FP-INJ-02: eval [linsert ...] must NOT fire W101; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_inj_02_eval_string_concat_still_w101() {
    // TP control: `eval "cmd $x"` does double substitution — W101 must fire.
    let src = "eval \"process $x\"";
    assert!(
        fires(src, D, "W101"),
        "FP-INJ-02 TP: eval of double-quoted string must fire W101; emitted {:?}",
        codes(src, D)
    );
}

// FP-INJ-03 — T102 suppression for HTTP::uri / HTTP::path (PATH_PREFIXED).

#[test]
fn fp_inj_03_http_uri_no_t102() {
    // FP-INJ-03: HTTP::uri always starts with `/`; cannot inject an option.
    let src = "set uri [HTTP::uri]\nregexp $uri test";
    assert!(
        !fires(src, IRULES, "T102"),
        "FP-INJ-03: HTTP::uri must NOT fire T102; emitted {:?}",
        codes(src, IRULES)
    );
}

#[test]
fn fp_inj_03_http_path_no_t102() {
    // FP-INJ-03: HTTP::path has the same path-anchoring guarantee.
    let src = "set p [HTTP::path]\nregexp $p test";
    assert!(
        !fires(src, IRULES, "T102"),
        "FP-INJ-03: HTTP::path must NOT fire T102; emitted {:?}",
        codes(src, IRULES)
    );
}

#[test]
fn fp_inj_03_path_prefixed_copy_suppresses() {
    // FP-INJ-03: PATH_PREFIXED colour propagates through copy assignments.
    let src = "set uri [HTTP::uri]\nset x $uri\nregexp $x test";
    assert!(
        !fires(src, IRULES, "T102"),
        "FP-INJ-03: PATH_PREFIXED copy must NOT fire T102; emitted {:?}",
        codes(src, IRULES)
    );
}

#[test]
fn fp_inj_03_literal_non_dash_prefix_no_t102() {
    // FP-INJ-03: a fixed non-dash literal prefix preserves path-prefixed safety.
    let src = "set foo \"path_[HTTP::path]\"\nregexp $foo test";
    assert!(
        !fires(src, IRULES, "T102"),
        "FP-INJ-03: non-dash literal prefix must NOT fire T102; emitted {:?}",
        codes(src, IRULES)
    );
}

// FP-INJ-04 — TP controls: literal `-` prefix and generic taint still warn.

#[test]
fn fp_inj_04_dash_prefix_still_warns() {
    // TP: prepending a fixed `-` breaks the path-anchoring guarantee.
    let src = "set foo \"-[HTTP::path]\"\nregexp $foo test";
    assert!(
        fires(src, IRULES, "T102"),
        "FP-INJ-04 TP: literal `-` prefix on HTTP::path must fire T102; emitted {:?}",
        codes(src, IRULES)
    );
}

#[test]
fn fp_inj_04_generic_taint_still_warns() {
    // TP: generic tainted data (no PATH_PREFIXED colour) still fires.
    let src = "set x [read $fd]\nregexp $x test";
    assert!(
        fires(src, IRULES, "T102"),
        "FP-INJ-04 TP: generic tainted data must fire T102; emitted {:?}",
        codes(src, IRULES)
    );
}

// FP-INJ-05 — `eval "$cmd $x"` (TP) → W101.
// The paired quick-fix rewrite (to `eval [list ...]`) is an LSP code-action
// contract, tested at the code-action layer rather than on the analyser
// diagnostic — see the lsp code-action tests.

const FP_INJ_05_REPRO: &str = "\
set x foo
eval \"process $x\"
";

#[test]
fn fp_inj_05_eval_string_fires_w101() {
    // TP: eval of a double-quoted command-string fires W101.
    assert!(
        fires(FP_INJ_05_REPRO, D, "W101"),
        "FP-INJ-05 TP: eval of double-quoted string must fire W101; emitted {:?}",
        codes(FP_INJ_05_REPRO, D)
    );
}

// FP-INJ-06 — a `PATH_PREFIXED` proof must not survive an unclassified
// (shape-changing) command it passes through — `TaintLattice::shape_unproven`.

#[test]
fn fp_inj_06_string_range_strips_path_prefixed_proof() {
    // FN regression: `string range` has no registry classification, so it
    // could turn a `/`-anchored path into something starting with `-`
    // (e.g. path `/-nocase/x`, `string range $p 1 end` => `-nocase/x`).
    // PATH_PREFIXED must not survive it.
    let src = "set p [HTTP::path]\nset stripped [string range $p 1 end]\nregexp $stripped test";
    assert!(
        fires(src, IRULES, "T102"),
        "FP-INJ-06: `string range` must strip PATH_PREFIXED, T102 must fire; emitted {:?}",
        codes(src, IRULES)
    );
}

#[test]
fn fp_inj_06_split_lindex_strips_path_prefixed_proof() {
    // FN regression, realistic iRules idiom: splitting `HTTP::path` and
    // indexing a segment can hand back attacker-controlled text with no
    // `/`-anchoring guarantee at all (path `/-nocase/x` splits to
    // `{{} -nocase x}`; `lindex $parts 1` is `-nocase`).
    let src =
        "set parts [split [HTTP::path] \"/\"]\nset second [lindex $parts 1]\nregexp $second test";
    assert!(
        fires(src, IRULES, "T102"),
        "FP-INJ-06: `split`/`lindex` must strip PATH_PREFIXED, T102 must fire; emitted {:?}",
        codes(src, IRULES)
    );
}

#[test]
fn fp_inj_06_pure_copy_still_suppresses() {
    // TN control: a *pure variable copy* (no command substitution at all)
    // is unaffected — `shape_unproven` only strips the proof for values
    // that passed through an unclassified command, not this simpler path
    // (already covered by FP-INJ-03, repeated here as a companion control
    // for the two repros above).
    let src = "set uri [HTTP::uri]\nset x $uri\nregexp $x test";
    assert!(
        !fires(src, IRULES, "T102"),
        "FP-INJ-06 control: plain copy must still suppress T102; emitted {:?}",
        codes(src, IRULES)
    );
}

// FP-INJ-07 — `exec -encoding <name>` must not mask the option-injection
// scan for the arguments that follow it — the registry was missing
// `-encoding` from `exec`'s option list entirely.

const FP_INJ_07_REPRO: &str = "\
proc run {} {
    set prog [gets stdin]
    exec -encoding utf-8 $prog
}
";

#[test]
fn fp_inj_07_exec_encoding_does_not_mask_option_scan() {
    // FN regression: `-encoding` takes a value (the encoding name); before
    // the registry declared that, `option_scan_region` treated the literal
    // encoding name as a definite positional and stopped scanning right
    // there, hiding every tainted argument after it (including the actual
    // program name) from T102/W304.
    assert!(
        fires(FP_INJ_07_REPRO, D, "T102"),
        "FP-INJ-07: `exec -encoding utf-8 $prog` must fire T102; emitted {:?}",
        codes(FP_INJ_07_REPRO, D)
    );
}

#[test]
fn fp_inj_07_exec_bare_control_still_warns() {
    // TP control: the same taint without the `-encoding` option already
    // fired correctly — pins the baseline this regression test compares
    // against.
    let src = "proc run {} {\n    set prog [gets stdin]\n    exec $prog\n}\n";
    assert!(
        fires(src, D, "T102"),
        "FP-INJ-07 control: bare `exec $prog` must fire T102; emitted {:?}",
        codes(src, D)
    );
}

// Note: `switch` lowers to its own dedicated `Statement::Switch` IR node
// (see `lowering/structured.rs::lower_switch`), not `Call` / `Barrier` /
// `AssignValue` — so its subject and pattern-list arguments never reach
// `emit_statement_warnings`'s sink dispatch at all, and T102 (like every
// other taint-sink code) cannot fire on them regardless of taint or of
// `reserved_trailing_words`. `reserved_trailing_words` is still correct,
// general registry data — it fixes the reachable W304 case above (FP-NAB-05)
// and will also cover T102 the day `switch`'s arguments become taint-visible
// — but a same-named T102 test here would credit it for behaviour that's
// actually caused by this separate, pre-existing statement-shape gap.
