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

//! STY family — style / usage warnings (W001/W104/W105/W113/W120/W122/W124/W126/
//! W201/W210/W212/W214/W216/W302/W306).
//! Pairs to `tests/test_fp_sty.py` and the §STY entries in `docs/design/compiler/FP.md`.

use super::{D, codes, fires};

// ---------------------------------------------------------------------------
// FP-STY-01 — W001 Tk geometry-manager shortcut form (grid/pack/place pathName)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_01_grid_pathname_no_w001() {
    // FP-STY-01: `grid .x` (no subcommand, just a pathName) is the Tk
    // geometry-manager shortcut form — must NOT fire W001.
    assert!(
        !fires("grid .x", D, "W001"),
        "FP-STY-01: grid .x shortcut must NOT fire W001; emitted: {:?}",
        codes("grid .x", D)
    );
}

#[test]
fn fp_sty_01_pack_pathname_no_w001() {
    // FP-STY-01: `pack .x` shortcut form.
    assert!(
        !fires("pack .x", D, "W001"),
        "FP-STY-01: pack .x shortcut must NOT fire W001; emitted: {:?}",
        codes("pack .x", D)
    );
}

#[test]
fn fp_sty_01_place_pathname_no_w001() {
    // FP-STY-01: `place .x` shortcut form.
    assert!(
        !fires("place .x", D, "W001"),
        "FP-STY-01: place .x shortcut must NOT fire W001; emitted: {:?}",
        codes("place .x", D)
    );
}

#[test]
fn fp_sty_01_genuine_unknown_subcommand_still_fires() {
    // TP control: a real unknown subcommand still fires W001.
    assert!(
        fires("grid bogus .x", D, "W001"),
        "FP-STY-01 TP: real unknown subcommand must still fire W001; emitted: {:?}",
        codes("grid bogus .x", D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-02 — W306 escaped `\[` / `\$` in quoted regexp patterns
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_02_escaped_bracket_no_w306() {
    // FP-STY-02: `regexp "\[abc\]" $s` — backslashes escape [ ] so they are
    // literal regex characters, NOT Tcl command substitution. Must not fire W306.
    let src = r#"regexp "\[abc\]" $s"#;
    assert!(
        !fires(src, D, "W306"),
        "FP-STY-02: escaped bracket in quoted regexp must NOT fire W306; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_02_escaped_dollar_no_w306() {
    // FP-STY-02: `regexp "\$end" $s` — backslash-dollar is a literal `$` in the
    // regex (an end-anchor), not a Tcl substitution. Must not fire W306.
    let src = r#"regexp "\$end" $s"#;
    assert!(
        !fires(src, D, "W306"),
        "FP-STY-02: escaped dollar in quoted regexp must NOT fire W306; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_02_live_dollar_in_quoted_pattern_still_fires() {
    // TP control: an UNescaped `$pat` in a quoted regex pattern IS a live
    // substitution and must still fire W306.
    let src = "regexp \"$pat\" $s";
    assert!(
        fires(src, D, "W306"),
        "FP-STY-02 TP: live $pat in quoted regexp pattern must fire W306; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_02_live_cmdsub_in_quoted_pattern_still_fires() {
    // TP control: a live `[clock seconds]` in a quoted regex pattern must still
    // fire W306.
    let src = "regexp \"[clock seconds]\" $s";
    assert!(
        fires(src, D, "W306"),
        "FP-STY-02 TP: live cmd-sub in quoted regexp pattern must fire W306; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-03 — W104 usage / template notation (`?arg?`, `<placeholder>`, `...`)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_03_optarg_question_marks_no_w104() {
    // FP-STY-03: `?optarg?` is the documented Tcl convention for optional
    // parameters in usage/help strings — must not fire W104.
    let src = "append usage \"?arg?\"";
    assert!(
        !fires(src, D, "W104"),
        "FP-STY-03: ?optarg? template notation must NOT fire W104; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_03_placeholder_angle_no_w104() {
    // FP-STY-03: `<placeholder>` template notation.
    let src = "append usage \"<placeholder>\"";
    assert!(
        !fires(src, D, "W104"),
        "FP-STY-03: <placeholder> template notation must NOT fire W104; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_03_ellipsis_no_w104() {
    // FP-STY-03: `...` continuation/varargs notation.
    let src = "append usage \"...\"";
    assert!(
        !fires(src, D, "W104"),
        "FP-STY-03: ellipsis notation must NOT fire W104; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_03_genuine_list_building_still_fires() {
    // TP control: a real string-concat list-building pattern still fires W104.
    let src = "proc f {items} {\n    foreach i $items {\n        append result \" $i\"\n    }\n}\n";
    assert!(
        fires(src, D, "W104"),
        "FP-STY-03 TP: genuine append result \" $i\" must still fire W104; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-04 — W126 non-channel value: lattice fix for lassign destructure
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_04_lassign_destructure_channels_no_w126() {
    // FP-STY-04: `lassign [chan pipe] ch wch` destructures the LIST result of
    // `chan pipe` into per-element channel-typed locals. Pre-fix the analyser typed
    // the lassign def-targets as LIST, causing W126 to fire when those locals were
    // later used as channels. Fix: lassign def-targets are typed UNKNOWN.
    let src = "lassign [chan pipe] ch wch\nputs $ch x";
    assert!(
        !fires(src, D, "W126"),
        "FP-STY-04: lassign destructure of chan pipe must NOT fire W126; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn tp_sty_04_puts_non_channel_literal_still_fires_w126() {
    // TP control for FP-STY-04: `puts` now declares its `channelId`
    // argument's position (via a dynamic `arg_role_resolver`, since the
    // optional leading `-nonewline` shifts it), so a value that provably
    // isn't a channel in that slot must still fire W126 — the fix above
    // only silences the *destructured, type-unknown* case, not a
    // genuinely wrong literal.
    let src = "puts \"not_a_channel\" hello";
    assert!(
        fires(src, D, "W126"),
        "TP: a non-channel literal in puts's channelId position must fire W126; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn tn_sty_04_puts_single_arg_is_content_not_channel() {
    // TN control: `puts hello` has only ONE positional arg — per Tcl
    // semantics that's the content string (written to stdout), not a
    // channel — so no Channel-role position applies and W126 must stay
    // silent (there is nothing to misclassify as a channel).
    let src = "puts hello";
    assert!(
        !fires(src, D, "W126"),
        "TN: puts's sole positional arg is content, not a channel; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-05 — W302 catch fire-and-forget (bare + subcommand-aware)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_05_bare_close_fire_and_forget_no_w302() {
    // FP-STY-05: `catch {close $fh}` is the documented Tcl idiom for
    // "close if open, otherwise ignore" — fire-and-forget cleanup. Must not fire W302.
    assert!(
        !fires("catch {close $fh}", D, "W302"),
        "FP-STY-05: catch {{close $fh}} fire-and-forget must NOT fire W302; emitted: {:?}",
        codes("catch {close $fh}", D)
    );
}

#[test]
fn fp_sty_05_ensemble_close_fire_and_forget_no_w302() {
    // FP-STY-05: `catch {chan close $fh}` is the same idiom in ensemble form.
    assert!(
        !fires("catch {chan close $fh}", D, "W302"),
        "FP-STY-05: catch {{chan close $fh}} must NOT fire W302; emitted: {:?}",
        codes("catch {chan close $fh}", D)
    );
}

#[test]
fn fp_sty_05_constructive_subcommand_still_fires() {
    // TP control: `catch {chan configure}` is constructive (sets an option, expects
    // success), not fire-and-forget; W302 must still fire.
    let src = "catch {chan configure $fh -opt val}";
    assert!(
        fires(src, D, "W302"),
        "FP-STY-05 TP: constructive subcommand must still fire W302; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_05_user_call_still_fires() {
    // TP control: `catch {my_proc $arg}` is a user call — generally needs a result
    // var; W302 fires.
    let src = "catch {my_proc $arg}";
    assert!(
        fires(src, D, "W302"),
        "FP-STY-05 TP: user-call catch must still fire W302; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-06 — W122/W124 OID-like dotted chains (not IPv4)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_06_oid_chain_no_w122_w124() {
    // FP-STY-06: `1.3.6.1.4.1.4203.1.11.3` (LDAP PEN OID) is an enterprise OID,
    // NOT an IPv4 address. The naive regex matched an embedded 4-component slice
    // where octet 4203 > 255; W122/W124 fired falsely.
    let src = "set oid 1.3.6.1.4.1.4203.1.11.3";
    assert!(
        !fires(src, D, "W122"),
        "FP-STY-06: OID chain must NOT fire W122; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W124"),
        "FP-STY-06: OID chain must NOT fire W124; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_06_real_ipv4_shaped_still_fires() {
    // TP control: a real IPv4-shaped literal with an out-of-range octet still
    // fires W124 (4-component dotted chain, no extension).
    let src = "set ip 192.168.4203.1";
    assert!(
        fires(src, D, "W124"),
        "FP-STY-06 TP: real IPv4-shaped out-of-range must still fire W124; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-07 — W120 package self-call (file is the provider)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_07_provider_self_call_no_w120() {
    // FP-STY-07: a file declaring `package provide msgcat 1.0` IS the
    // implementation of msgcat; subsequent `msgcat::mc` calls inside that file
    // don't need a `package require msgcat`.
    let src = "package provide msgcat 1.0\nmsgcat::mc hello";
    assert!(
        !fires(src, D, "W120"),
        "FP-STY-07: package provide + self-call must NOT fire W120; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_07_no_provide_still_fires() {
    // TP control: using a package's command without either a require or a provide
    // must still fire W120.
    let src = "msgcat::mc hello";
    assert!(
        fires(src, D, "W120"),
        "FP-STY-07 TP: missing package require must still fire W120; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-08 — W214 empty-body proc stubs
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_08_empty_body_stub_no_w214() {
    // FP-STY-08: `proc stub {a b} {}` is the canonical Tcl signature-stub pattern
    // — every parameter is necessarily "unused" because there is no body.
    // Flagging W214 on every param is noise.
    let src = "proc stub {a b} {}";
    assert!(
        !fires(src, D, "W214"),
        "FP-STY-08: empty-body stub proc must NOT fire W214; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_08_non_empty_body_unused_param_still_fires() {
    // TP control: a proc with a non-empty body but a truly unused parameter still
    // fires W214. We check that W214 fires with 'b' in the message.
    let src = "proc f {a b} { return $a }";
    let all = codes(src, D);
    let w214_b = all.iter().any(|c| c == "W214");
    // The Rust analyser may not embed the param name in the code string — just
    // check W214 fires at all (param b is unused).
    assert!(
        w214_b,
        "FP-STY-08 TP: non-empty body with unused 'b' must still fire W214; emitted: {all:?}"
    );
}

#[test]
fn fp_sty_08_snit_quoted_keyword_marker_no_w214() {
    // FP-STY-08: snit-style DSL uses `{"as" ""}` as a positional keyword marker —
    // the parameter name is the literal `"as"` and the body can't reference it as
    // a variable. Detect via param name starting + ending with `"` and exempt from W214.
    let src = "proc xyz {\"as\" v} { return $v }";
    assert!(
        !fires(src, D, "W214"),
        "FP-STY-08: quoted-keyword marker \"as\" must NOT fire W214; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_08_bare_keyword_param_still_fires() {
    // TP control: the same shape with NO quote marker (`proc xyz {as v}`) MUST
    // still fire W214 on the unused `as`.
    let src = "proc xyz {as v} { return $v }";
    assert!(
        fires(src, D, "W214"),
        "FP-STY-08 TP: bare unused 'as' param (no quote marker) must still fire W214; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-09 — W214 dispatcher needs arity-compatible peer (D3-P9/D4-F4)
// ---------------------------------------------------------------------------

const FP_STY_09_REPRO: &str = "\
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
    proc unrelated {cmd} { $cmd x }
}
";

#[test]
fn fp_sty_09_arity_incompatible_dispatcher_does_not_suppress() {
    // TP: a 1-arg `$cmd x` dispatcher in the same namespace as 2-arg peers a/b/c
    // is NOT evidence for the peer protocol — arities don't match. W214 must still
    // fire 3 times on `token`.
    let all = codes(FP_STY_09_REPRO, D);
    let w214_count = all.iter().filter(|c| c.as_str() == "W214").count();
    assert!(
        w214_count >= 3,
        "FP-STY-09 TP: W214 must fire on 'token' in all three peers (a/b/c); the 1-arg \
         unrelated dispatcher is arity-incompatible. Got {w214_count} W214: {all:?}"
    );
}

#[test]
fn fp_sty_09_arity_compatible_dispatcher_suppresses() {
    // TN control: when a same-namespace dispatcher's arity matches the peer
    // signature (2-arg `$cmd $ctx $token`), the protocol-match heuristic correctly
    // applies and W214 stays silent.
    let src = "\
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
    proc dispatch {cmd ctx token} { $cmd $ctx $token }
}
";
    assert!(
        !fires(src, D, "W214"),
        "FP-STY-09: 2-arg dispatcher must suppress 2-arg peer family W214; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-10 — scan_provably_no_match soundness (%n / Inf / format whitespace, D4-F1)
// ---------------------------------------------------------------------------

const FP_STY_10_REPRO: &str = "proc f {} { scan {} %n n; puts $n }\n";

#[test]
fn fp_sty_10_scan_percent_n_on_empty_input_silent() {
    // FP-STY-10: `scan "" %n n` ALWAYS succeeds (writes the count of consumed
    // characters with no input required); reading `$n` after must NOT fire W210.
    assert!(
        !fires(FP_STY_10_REPRO, D, "W210"),
        "FP-STY-10: scan %n always-match must NOT fire W210; emitted: {:?}",
        codes(FP_STY_10_REPRO, D)
    );
}

#[test]
fn fp_sty_10_scan_float_accepts_inf_silent() {
    // FP-STY-10: `scan Inf %f f` succeeds in tclsh (Tcl_GetDouble accepts
    // Inf/Infinity/NaN). Must not fire W210.
    let src = "proc f {} { scan Inf %f f; puts $f }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-STY-10: scan %f with Inf must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_10_scan_format_whitespace_cr_silent() {
    // FP-STY-10: format `\r` is whitespace that skips input whitespace;
    // `scan " 123" "\r%d" n` parses 123 successfully. Must not fire W210.
    let src = "proc f {} { scan { 123} \"\\r%d\" n; puts $n }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-STY-10: scan format \\r whitespace must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_10_scan_genuine_no_match_still_fires() {
    // TP control: a real provable no-match (`scan abc %d n`) must still fire W210.
    let src = "proc f {} { scan abc %d n; puts $n }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-STY-10 TP: real provable no-match (abc vs %d) must still fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-11 — variadic var-write resolver for scan / lassign / binary scan (D4-F2)
// ---------------------------------------------------------------------------

const FP_STY_11_REPRO: &str = "\
proc f {} {
    scan {x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14 x15 x16 x17 x18 x19} {%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} v0 v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v15 v16 v17 v18 v19
    return $v19
}
";

#[test]
fn fp_sty_11_scan_20_vars_no_false_w210() {
    // FP-STY-11: pre-fix the scan spec hard-coded VAR_WRITE for slots [2..19]
    // only; the 20th var (v19) wasn't recognised and the subsequent `return $v19`
    // falsely fired W210. D4-F2 closure adds the dynamic arg_role_resolver.
    assert!(
        !fires(FP_STY_11_REPRO, D, "W210"),
        "FP-STY-11: scan with 20 vars must not false-fire W210 on v18/v19; emitted: {:?}",
        codes(FP_STY_11_REPRO, D)
    );
}

#[test]
fn fp_sty_11_lassign_many_vars_no_false_w210() {
    // FP-STY-11: same fix for `lassign` — all positional args after the list arg
    // are classified VAR_WRITE.
    let src =
        "proc f {l} { lassign $l a b c d e f g h i j k l2 m n o p q r s t u v w x y; return $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-STY-11: lassign with many vars must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_11_binary_scan_many_vars_no_false_w210() {
    // FP-STY-11: same fix for `binary scan`.
    let src = "proc f {} { binary scan {} {i i i i i i i i i i i i i i i i i i i i} a b c d e f g h i j k l m n o p q r s t; return $t }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-STY-11: binary scan with 20 vars must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_11_scan_fewer_specifiers_than_vars_still_fires() {
    // TP control: when scan has fewer %-directives than var slots, the second var
    // IS provably unset and the read must fire W210.
    let src = "proc f {} { scan abc %s v1; return $v2 }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-STY-11 TP: scan with one %s but `return $v2` must fire W210 on v2; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-12 — W216 / W212 braced indirect-array-element idiom ${var}(idx)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_12_set_indirect_array_no_w216_w212() {
    // FP-STY-12: `set ${token}(status) eof` where `token` holds an array name is
    // the canonical indirect-array-element write. Both W216 and W212 must be silent.
    let src = "set token ::http::1\nset ${token}(status) eof\n";
    assert!(
        !fires(src, D, "W216"),
        "FP-STY-12: indirect-array set must NOT fire W216; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W212"),
        "FP-STY-12: indirect-array set must NOT fire W212; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_12_info_exists_indirect_no_w216_w212() {
    // FP-STY-12: `info exists ${token}(-pipeline)` indirect-array read.
    let src = "info exists ${token}(-pipeline)\n";
    assert!(
        !fires(src, D, "W216"),
        "FP-STY-12: info exists indirect-array must NOT fire W216; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W212"),
        "FP-STY-12: info exists indirect-array must NOT fire W212; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_12_unset_indirect_no_w216() {
    // FP-STY-12: `unset ${tok}(socketcoro)` — unset takes a varname, so the
    // indirect-array form is the intended element unset.
    let src = "unset ${tok}(socketcoro)\n";
    assert!(
        !fires(src, D, "W216"),
        "FP-STY-12: unset indirect-array must NOT fire W216; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W212"),
        "FP-STY-12: unset indirect-array must NOT fire W212; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_12_vwait_incr_append_lappend_indirect_no_w216() {
    // FP-STY-12: every varname-taking command honours the indirect idiom.
    for src in [
        "vwait ${token}(status)\n",
        "incr ${arr}(n)\n",
        "append ${arr}(buf) x\n",
        "lappend ${arr}(list) item\n",
    ] {
        assert!(
            !fires(src, D, "W216"),
            "FP-STY-12: W216 should be silent on {:?}; emitted: {:?}",
            src,
            codes(src, D)
        );
        assert!(
            !fires(src, D, "W212"),
            "FP-STY-12: W212 should be silent on {:?}; emitted: {:?}",
            src,
            codes(src, D)
        );
    }
}

#[test]
fn fp_sty_12_value_position_still_fires_w216() {
    // TP control: in a *value* position `${arr}(x)` is a broken read for
    // `$arr(x)` (Tcl errors `variable is array`), so W216 still fires.
    assert!(
        fires("puts ${arr}(x)\n", D, "W216"),
        "FP-STY-12 TP: value-position ${{arr}}(x) in puts must fire W216; emitted: {:?}",
        codes("puts ${arr}(x)\n", D)
    );
    assert!(
        fires("set y ${arr}(x)\n", D, "W216"),
        "FP-STY-12 TP: value-position ${{arr}}(x) in set must fire W216; emitted: {:?}",
        codes("set y ${arr}(x)\n", D)
    );
}

#[test]
fn fp_sty_12_bare_dollar_name_still_fires_w212() {
    // TP control: bare `set $x` and `set $arr(idx)` are genuine dynamic-name
    // foot-guns (no braces → not the indirect-array idiom).
    assert!(
        fires("set $x v\n", D, "W212"),
        "FP-STY-12 TP: set $x must still fire W212; emitted: {:?}",
        codes("set $x v\n", D)
    );
    assert!(
        fires("set $arr(idx) v\n", D, "W212"),
        "FP-STY-12 TP: set $arr(idx) must still fire W212; emitted: {:?}",
        codes("set $arr(idx) v\n", D)
    );
}

#[test]
fn fp_sty_12_index_less_brace_still_fires_w212() {
    // TP control: `set ${x} v` (no `(idx)` suffix) is the dynamic-name foot-gun,
    // not the indirect-array idiom, so W212 still fires.
    assert!(
        fires("set ${x} v\n", D, "W212"),
        "FP-STY-12 TP: set ${{x}} must still fire W212; emitted: {:?}",
        codes("set ${x} v\n", D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-13 — W113 redefining an overridable Tcl library procedure
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_13_unknown_override_no_w113() {
    // FP-STY-13: `proc unknown` is the documented Tcl extension hook (a Tcl
    // library proc, not a C built-in) — redefining it is supported, so no W113.
    assert!(
        !fires("proc unknown args { return }\n", D, "W113"),
        "FP-STY-13: proc unknown override must NOT fire W113; emitted: {:?}",
        codes("proc unknown args { return }\n", D)
    );
}

#[test]
fn fp_sty_13_library_procs_no_w113() {
    // FP-STY-13: the overridable Tcl *library* procedures (defined in init.tcl /
    // auto.tcl / history.tcl / package.tcl / word.tcl) are not C built-ins and
    // must not fire W113 when redefined.
    for (name, params) in [
        ("history", "{args}"),
        ("auto_execok", "name"),
        ("auto_load", "{cmd {ns {}}}"),
        ("auto_mkindex", "{dir args}"),
        ("auto_qualify", "{cmd ns}"),
        ("auto_reset", "{}"),
        ("parray", "{a {pattern *}}"),
        ("pkg_mkIndex", "args"),
        ("tcl_findLibrary", "{a b c d e f}"),
        ("tcl_wordBreakAfter", "{str start}"),
        ("tcl_endOfWord", "{str start}"),
    ] {
        let src = format!("proc {name} {params} {{ return }}\n");
        assert!(
            !fires(&src, D, "W113"),
            "FP-STY-13: W113 should be silent on {name:?}; emitted: {:?}",
            codes(&src, D)
        );
    }
}

#[test]
fn fp_sty_13_c_builtin_still_fires_w113() {
    // TP control: redefining a genuine C built-in (byte-compiled) still fires.
    assert!(
        fires("proc set {a b} { return }\n", D, "W113"),
        "FP-STY-13 TP: proc set must fire W113; emitted: {:?}",
        codes("proc set {a b} { return }\n", D)
    );
    assert!(
        fires("proc puts {a} { return }\n", D, "W113"),
        "FP-STY-13 TP: proc puts must fire W113; emitted: {:?}",
        codes("proc puts {a} { return }\n", D)
    );
}

#[test]
fn fp_sty_13_non_bytecompiled_c_command_still_fires_w113() {
    // TP control: C commands that are *not* byte-compiled but dangerous to
    // redefine (clock/after/socket/glob) must NOT be swept up by the
    // library-proc exemption — they keep firing W113.
    for name in ["clock", "after", "socket", "glob"] {
        let src = format!("proc {name} {{a}} {{ return }}\n");
        assert!(
            fires(&src, D, "W113"),
            "FP-STY-13 TP: proc {name} must still fire W113; emitted: {:?}",
            codes(&src, D)
        );
    }
}

// ---------------------------------------------------------------------------
// FP-STY-14 — W105 single bare-variable body is a script reference
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_14_eval_single_var_body_no_w105() {
    // FP-STY-14: `eval $cmd` — the variable holds the script; bracing it
    // (`eval {$cmd}`) would evaluate the literal text `$cmd`, so the W105
    // brace-fix is wrong and must not fire.
    assert!(
        !fires("eval $cmd", D, "W105"),
        "FP-STY-14: eval $cmd must NOT fire W105; emitted: {:?}",
        codes("eval $cmd", D)
    );
}

#[test]
fn fp_sty_14_callback_dispatch_body_no_w105() {
    // FP-STY-14: `namespace eval :: $state(-command) $token` is registered-callback
    // dispatch; W105 must not double-flag it at Error severity.
    let src = "namespace eval :: $state(-command) $token";
    assert!(
        !fires(src, D, "W105"),
        "FP-STY-14: callback dispatch body must NOT fire W105; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_14_dynamic_proc_and_after_no_w105() {
    // FP-STY-14: every single-bare-variable body is a script reference.
    for src in [
        "proc $fakeName $arglist $body",
        "after 0 $coroName",
        "interp eval $child $contents",
        "foreach name $nameList $body",
        "uplevel $script",
    ] {
        assert!(
            !fires(src, D, "W105"),
            "FP-STY-14: W105 should be silent on {:?}; emitted: {:?}",
            src,
            codes(src, D)
        );
    }
}

#[test]
fn fp_sty_14_quoted_interpolated_body_still_fires() {
    // TP control: a *quoted* body with interpolation (`eval "do $script"`) really
    // is an inline script being woven from substitutions — it can and should be
    // braced, so W105 still fires.
    let src = "eval \"do $script\"";
    assert!(
        fires(src, D, "W105"),
        "FP-STY-14 TP: quoted interpolated body must fire W105; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_14_composite_body_still_fires() {
    // TP control: a composite body (var + concatenation) still fires — it is not a
    // single bare reference.
    assert!(
        fires("eval $cmd$args", D, "W105"),
        "FP-STY-14 TP: composite body must fire W105; emitted: {:?}",
        codes("eval $cmd$args", D)
    );
    assert!(
        fires("fileevent $s readable ${t}--Coro", D, "W105"),
        "FP-STY-14 TP: composite body ${{t}}--Coro must fire W105; emitted: {:?}",
        codes("fileevent $s readable ${t}--Coro", D)
    );
}

#[test]
fn fp_sty_14_uplevel_body_participates_in_w105_like_eval() {
    // Issue #837: now that `uplevel`'s script word carries `ArgRole::Body`, the
    // W105 unbraced-body check applies to it exactly as it does to `eval`.
    // TP: a quoted interpolated body past the level word is an inline script
    // woven from substitutions — brace it.
    assert!(
        fires("uplevel 1 \"do $script\"", D, "W105"),
        "FP-STY-14: an unbraced substituted uplevel body must fire W105; emitted: {:?}",
        codes("uplevel 1 \"do $script\"", D)
    );
    // FP: a braced body is already correct — no W105.
    assert!(
        !fires("uplevel 1 {do $script}", D, "W105"),
        "FP-STY-14: a braced uplevel body must NOT fire W105; emitted: {:?}",
        codes("uplevel 1 {do $script}", D)
    );
    // FP: a single bare-variable body (`uplevel $script`, implicit level 1) is
    // a script reference, not an inline block — the brace-fix would be wrong.
    assert!(
        !fires("uplevel $script", D, "W105"),
        "FP-STY-14: a single bare-variable uplevel body must NOT fire W105; emitted: {:?}",
        codes("uplevel $script", D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-15 — lexer: $ before a closing " merged the quoted word with the next
// (E002 / E205 / W306)
// ---------------------------------------------------------------------------
//
// NOTE: the direct lexer API test for this word-boundary case has no
// diagnostic counterpart and is skipped here (bridge-only; 1 test omitted).

#[test]
fn fp_sty_15_regsub_dollar_anchor_no_errors() {
    // FP-STY-15: `regsub "\n$" $msg "" out` — `"\n$"` is a literal regex
    // end-of-line anchor ($ before " is not a substitution). Pre-fix the lexer
    // merged the closing quote with $msg, raising E002/E205/W306 on valid Tcl.
    let src = r#"regsub "\n$" $msg "" out"#;
    let all = codes(src, D);
    assert!(
        !all.iter().any(|c| c == "E002"),
        "FP-STY-15: regsub dollar-anchor must NOT fire E002; emitted: {all:?}"
    );
    assert!(
        !all.iter().any(|c| c == "E205"),
        "FP-STY-15: regsub dollar-anchor must NOT fire E205; emitted: {all:?}"
    );
    assert!(
        !all.iter().any(|c| c == "W306"),
        "FP-STY-15: regsub dollar-anchor must NOT fire W306; emitted: {all:?}"
    );
}

#[test]
fn fp_sty_15_string_match_dollar_anchor_no_arity() {
    // FP-STY-15: `string match "abc$" $x` is two args after `match`; the tail
    // `$` must not merge `"abc$"` with `$x` into one word (E002).
    let src = "string match \"abc$\" $x";
    assert!(
        !fires(src, D, "E002"),
        "FP-STY-15: string match dollar-anchor must NOT fire E002; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_15_regex_end_anchor_no_w306() {
    // FP-STY-15: `regexp -- "^foo$" $text` — the `$` is the literal end-anchor,
    // not a substitution, so W306 must not fire.
    let src = "regexp -- \"^foo$\" $text";
    assert!(
        !fires(src, D, "W306"),
        "FP-STY-15: regex end-anchor must NOT fire W306; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_15_live_var_in_quoted_pattern_still_w306() {
    // TP control: a genuine live `$bar` (b is a name char) in a quoted regex
    // pattern is the foot-gun and still fires W306.
    let src = "regexp -- \"^foo$bar\" $text";
    assert!(
        fires(src, D, "W306"),
        "FP-STY-15 TP: live $bar in quoted regex pattern must fire W306; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-16 — W201 manual-path-concat on prose / protocol / display strings
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_16_http_request_line_no_w201() {
    // FP-STY-16: `"CONNECT $host:$port HTTP/1.1"` is an HTTP request line — the
    // `/` is the protocol version, not a path separator. `[file join]` would shred
    // it on the spaces, so W201 must not fire.
    let src = "set bypass \"CONNECT $host:$port HTTP/1.1\"";
    assert!(
        !fires(src, D, "W201"),
        "FP-STY-16: HTTP request line must NOT fire W201; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_16_usage_message_no_w201() {
    // FP-STY-16: a usage / display message is not a filesystem path.
    let src = "set msg \"Usage: [file tail $exe] script \"";
    assert!(
        !fires(src, D, "W201"),
        "FP-STY-16: usage message must NOT fire W201; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_16_prose_with_path_no_w201() {
    // FP-STY-16: prose that merely mentions a path is display text, not path
    // construction.
    let src = "set x \"see $dir/readme for help\"";
    assert!(
        !fires(src, D, "W201"),
        "FP-STY-16: prose with path mention must NOT fire W201; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_16_genuine_path_concat_still_fires() {
    // TP control: a real path-building concat with no literal whitespace still
    // fires W201.
    assert!(
        fires("set f \"$dir/$name\"", D, "W201"),
        "FP-STY-16 TP: genuine path concat must fire W201; emitted: {:?}",
        codes("set f \"$dir/$name\"", D)
    );
    assert!(
        fires("set p $root/sub/$file", D, "W201"),
        "FP-STY-16 TP: genuine path concat must fire W201; emitted: {:?}",
        codes("set p $root/sub/$file", D)
    );
}

#[test]
fn fp_sty_16_path_with_command_sub_still_fires() {
    // TP control: `set f "$dir/[file tail $path]"` — the command sub's *internal*
    // spaces are inside one CMD token and must NOT suppress; this is a genuine path
    // concat and still fires W201.
    let src = "set f \"$dir/[file tail $path]\"";
    assert!(
        fires(src, D, "W201"),
        "FP-STY-16 TP: path concat with command-sub segment must still fire W201; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-17 — W001 same-file shadow suppression (proc / class / alias /
// ensemble / stub redefining a registry ensemble command)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_17_proc_shadow_no_w001() {
    // FP-STY-17: a same-file `proc string {...}` completely replaces the
    // builtin ensemble at the call site — `string reverse hello` resolves to
    // the user proc, not the registry's `string` subcommand set, so it must
    // NOT fire W001, exactly as the sibling E002/E003 arity check already
    // abstains for a shadowed builtin.
    let src = "proc string {op args} { return \"shadowed:$op\" }\nstring reverse hello\n";
    assert!(
        !fires(src, D, "W001"),
        "FP-STY-17: proc-shadowed ensemble call must NOT fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_17_alias_shadow_no_w001() {
    // FP-STY-17: `interp alias {} info {} myInfo` points `info` at an alias
    // target — a call through it must not be checked against the registry
    // `info` ensemble's subcommand set.
    let src = "proc myInfo {op args} { return $op }\ninterp alias {} info {} myInfo\ninfo bogus\n";
    assert!(
        !fires(src, D, "W001"),
        "FP-STY-17: alias-shadowed ensemble call must NOT fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_17_class_shadow_no_w001() {
    // FP-STY-17: `oo::class create array {...}` makes `array` the class's own
    // object command (dispatching `new`/`create`, not the registry `array`
    // ensemble's subcommand set) — `array create obj` must not fire W001
    // even though `create` is not a real `array` subcommand.
    let src = "oo::class create array {\n    constructor {} {}\n}\narray create obj\n";
    assert!(
        !fires(src, D, "W001"),
        "FP-STY-17: class-shadowed ensemble call must NOT fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_17_stub_shadow_no_w001() {
    // FP-STY-17: an inline `# tcl-lsp: stub` declaration is a document-global
    // promise that the command exists (e.g. injected by a test harness or a
    // package the analyser doesn't see the source of) — it must suppress
    // W001 exactly like it suppresses the arity checks.
    let src = "# tcl-lsp: stub string\nstring reverse hello\n";
    assert!(
        !fires(src, D, "W001"),
        "FP-STY-17: stub-shadowed ensemble call must NOT fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_17_unshadowed_command_still_fires() {
    // TP control: shadowing `string` must not blind the analyser to a
    // genuine unknown subcommand on a *different*, unshadowed ensemble in
    // the same file.
    let src = "proc string {op args} { return $op }\ninfo bogus\n";
    assert!(
        fires(src, D, "W001"),
        "FP-STY-17 TP: an unshadowed ensemble must still fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_17_call_before_shadow_definition_still_fires() {
    // TP control: Tcl executes top-level commands in source order during
    // load, so a call *before* the shadowing proc's definition still reaches
    // the real builtin and still errors — confirmed against tclsh 8.6.14:
    // `catch {string mach x y} e` before `proc string {...}` still raises
    // "unknown or ambiguous subcommand". W001 must still fire here.
    let src = "string mach hello world\nproc string {op args} { return $op }\n";
    assert!(
        fires(src, D, "W001"),
        "FP-STY-17 TP: a call before the shadowing definition must still fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_17_proc_body_call_shadowed_by_later_def_no_w001() {
    // FP-STY-17: unlike a top-level call, a call inside a proc body runs
    // only after the whole script has loaded — a `proc string {...}`
    // appearing *after* the caller's own definition still shadows it, so no
    // W001.
    let src =
        "proc caller {} {\n    string reverse hello\n}\nproc string {op args} { return $op }\n";
    assert!(
        !fires(src, D, "W001"),
        "FP-STY-17: a proc-body call shadowed by a later definition must NOT fire W001; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-18 — W001 `{*}`-expanded subcommand position (`cmd {*}{subcmd args…}`)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_18_expanded_literal_subcommand_no_w001() {
    // FP-STY-18: `{*}{create a b}` splices the *elements* `create`, `a`, `b`
    // into the argument list — confirmed against tclsh 8.6.14: `dict
    // {*}{create a b}` evaluates `dict create a b`. The raw source text
    // `"create a b"` must never be compared against the subcommand set.
    let src = "dict {*}{create a b}";
    assert!(
        !fires(src, D, "W001"),
        "FP-STY-18: {{*}}-expanded literal subcommand must NOT fire W001; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_sty_18_genuine_unknown_subcommand_without_expansion_still_fires() {
    // TP control: the same subcommand set, unexpanded, still fires on a
    // genuine unknown subcommand.
    assert!(
        fires("dict bogus a b", D, "W001"),
        "FP-STY-18 TP: a genuine unknown subcommand must still fire W001; emitted: {:?}",
        codes("dict bogus a b", D)
    );
}

// ---------------------------------------------------------------------------
// FP-STY-19 — missing Tk 9.0 subcommands (`wm iconbadge`, `grid`/`pack`/
// `place content`)
// ---------------------------------------------------------------------------

#[test]
fn fp_sty_19_wm_iconbadge_not_unknown() {
    // FP-STY-19: `wm iconbadge` is a genuine Tk 9.0+ subcommand (wm.n) that
    // was missing from the registry entirely, so it drew a spurious
    // "Unknown subcommand" under every dialect (Tk subcommands are checked
    // regardless of the active Tcl dialect). Under tcl9.0 it is fully known.
    assert!(
        !fires("wm iconbadge .win 5", "tcl9.0", "W001"),
        "FP-STY-19: wm iconbadge must NOT fire W001 under tcl9.0; emitted: {:?}",
        codes("wm iconbadge .win 5", "tcl9.0")
    );
    // Under tcl8.6 it is a genuine subcommand that doesn't exist before 9.0
    // — the correct diagnostic is W002 (disabled in this dialect profile),
    // never W001, mirroring the `package files` / tcl8.6 precedent.
    assert!(
        !fires("wm iconbadge .win 5", D, "W001"),
        "FP-STY-19: wm iconbadge must NOT fire W001 under tcl8.6; emitted: {:?}",
        codes("wm iconbadge .win 5", D)
    );
    assert!(
        fires("wm iconbadge .win 5", D, "W002"),
        "FP-STY-19: wm iconbadge under tcl8.6 must fire W002 (disabled in dialect); emitted: {:?}",
        codes("wm iconbadge .win 5", D)
    );
}

#[test]
fn fp_sty_19_grid_pack_place_content_not_unknown() {
    // FP-STY-19: Tk 9.0 renamed `slaves` to `content` (grid.n/pack.n/
    // place.n) as the canonical spelling; it was missing from the registry
    // entirely for all three geometry managers.
    for src in [
        "grid content .frame",
        "pack content .frame",
        "place content .frame",
    ] {
        assert!(
            !fires(src, "tcl9.0", "W001"),
            "FP-STY-19: {src:?} must NOT fire W001 under tcl9.0; emitted: {:?}",
            codes(src, "tcl9.0")
        );
    }
}

#[test]
fn fp_sty_19_genuine_unknown_wm_and_geometry_subcommands_still_fire() {
    // TP control: real typos on the same commands still fire.
    assert!(
        fires("wm bogus .win", "tcl9.0", "W001"),
        "FP-STY-19 TP: wm bogus must still fire W001; emitted: {:?}",
        codes("wm bogus .win", "tcl9.0")
    );
    assert!(
        fires("grid bogus .frame", "tcl9.0", "W001"),
        "FP-STY-19 TP: grid bogus must still fire W001; emitted: {:?}",
        codes("grid bogus .frame", "tcl9.0")
    );
}
