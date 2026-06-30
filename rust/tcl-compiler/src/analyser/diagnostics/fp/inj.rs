//! INJ family — injection / style (W301 uplevel, W101 eval-injection, T102
//! option-injection taint).
//! Pairs to `tests/test_fp_inj.py` and the §INJ entries in `docs/design/compiler/FP.md`.
//!
//! The T102 cases are iRules-dialect (`HTTP::uri` / `HTTP::path` taint sources),
//! so they run under the `f5-irules` dialect rather than the default `D`.

use super::{D, codes, fires};

const IRULES: &str = "f5-irules";

// ---------------------------------------------------------------------------
// FP-INJ-01 — `uplevel 1 $body` is the safe canonical idiom.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// FP-INJ-02 — `eval [list ...]` is the canonical safe form.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// FP-INJ-03 — T102 suppression for HTTP::uri / HTTP::path (PATH_PREFIXED).
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// FP-INJ-04 — TP controls: literal `-` prefix and generic taint still warn.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// FP-INJ-05 — `eval "$cmd $x"` (TP) → W101.
// The paired quick-fix rewrite (to `eval [list ...]`) is an LSP code-action
// contract, tested at the code-action layer rather than on the analyser
// diagnostic — see the lsp code-action tests.
// ---------------------------------------------------------------------------

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
