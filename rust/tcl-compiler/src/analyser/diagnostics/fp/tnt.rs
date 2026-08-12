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

//! TNT family — taint (T100 expr-operand, T101 puts-content, T102 option,
//! T104 network-sink, T105 interp-eval), position-aware sink filtering.

use super::{D, codes, fires};

// ---------------------------------------------------------------------------
// FP-TNT-01 — T100 direct-operand expr filter.
// ---------------------------------------------------------------------------

const FP_TNT_01_REPRO: &str = "\
proc f {} {
    set data [gets stdin]
    # $data is consumed as an argument to `string length`, not re-parsed
    # as expr text. The cmd-sub boundary protects it -- no T100.
    expr {[string length $data] / 8}
}
";

#[test]
fn fp_tnt_01_cmd_sub_arg_position_no_t100() {
    // FP: tainted value inside a cmd-sub argument is not a direct expr operand.
    assert!(
        !fires(FP_TNT_01_REPRO, D, "T100"),
        "FP-TNT-01: cmd-sub arg must NOT fire T100; emitted {:?}",
        codes(FP_TNT_01_REPRO, D)
    );
}

#[test]
fn fp_tnt_01_direct_operand_still_fires() {
    // TP: a tainted value as a DIRECT expr operand fires T100 (numeric coercion).
    let src = "proc f {} {\n    set data [gets stdin]\n    expr {$data + 1}\n}";
    assert!(
        fires(src, D, "T100"),
        "FP-TNT-01 TP: direct operand must fire T100; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_01_function_arg_direct_operand_still_fires() {
    // TP: `expr {abs($data)}` — direct argument of an expr-level function.
    let src = "proc f {} {\n    set data [gets stdin]\n    expr {abs($data)}\n}";
    assert!(
        fires(src, D, "T100"),
        "FP-TNT-01 TP: abs($data) must fire T100; emitted {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-TNT-02 — T101 puts channel-position filter.
// ---------------------------------------------------------------------------

const FP_TNT_02_REPRO: &str = "\
proc f {} {
    set chan [gets stdin]
    # $chan is the destination channel id, NOT the content; T101 must
    # not fire on a positional channel-id arg.
    puts -nonewline $chan \"hello\\n\"
}
";

#[test]
fn fp_tnt_02_channel_id_position_no_t101() {
    // FP: a tainted channel id is not output-injection.
    assert!(
        !fires(FP_TNT_02_REPRO, D, "T101"),
        "FP-TNT-02: channel-id position must NOT fire T101; emitted {:?}",
        codes(FP_TNT_02_REPRO, D)
    );
}

#[test]
fn fp_tnt_02_content_arg_still_fires() {
    // TP: a tainted content arg to puts fires T101.
    let src = "proc f {} {\n    set data [gets stdin]\n    puts $data\n}";
    assert!(
        fires(src, D, "T101"),
        "FP-TNT-02 TP: tainted puts content must fire T101; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_02_content_with_channel_still_fires() {
    // TP: `puts $chan $data` — $data is content, still fires T101.
    let src =
        "proc f {} {\n    set chan stdout\n    set data [gets stdin]\n    puts $chan $data\n}";
    assert!(
        fires(src, D, "T101"),
        "FP-TNT-02 TP: content arg with channel must fire T101; emitted {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-TNT-03 — eval/uplevel/interp-eval LIST_CANONICAL position filter.
// ---------------------------------------------------------------------------

#[test]
fn fp_tnt_03_eval_list_tainted_head_fires_t100() {
    // TP: `eval [list $raw]` — tainted var at list index 0 (command word).
    let src = "set raw [gets stdin]\neval [list $raw]\n";
    assert!(
        fires(src, D, "T100"),
        "FP-TNT-03 TP: tainted [list ...] head must fire T100; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_03_eval_list_literal_head_no_t100() {
    // TN: `eval [list puts $raw]` — literal cmd at index 0, tainted at index 1.
    let src = "set raw [gets stdin]\neval [list puts $raw]\n";
    assert!(
        !fires(src, D, "T100"),
        "FP-TNT-03 TN: [list <known-cmd> $raw] must suppress T100; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_03_uplevel_list_tainted_head_fires_t100() {
    // TP: `uplevel [list $raw]` runs $raw as the command word in the caller frame.
    let src = "set raw [gets stdin]\nuplevel [list $raw]\n";
    assert!(
        fires(src, D, "T100"),
        "FP-TNT-03 TP: uplevel [list $raw] must fire T100; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_03_eval_list_via_var_still_fires() {
    // TP: `set lst [list $raw]; eval $lst` — the [list ...] is not visible at the
    // eval site, so the suppression must not apply.
    let src = "set raw [gets stdin]\nset lst [list $raw]\neval $lst\n";
    assert!(
        fires(src, D, "T100"),
        "FP-TNT-03 TP: propagated [list]-wrapped taint must fire T100; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_03_interp_eval_list_tainted_head_fires_t105() {
    // TP: `interp eval $child [list $raw]` — cross-interp eval hazard.
    let src = "set raw [gets stdin]\ninterp eval $child [list $raw]\n";
    assert!(
        fires(src, D, "T105"),
        "FP-TNT-03 TP: interp eval [list $raw] must fire T105; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_03_interp_eval_list_literal_head_no_t105() {
    // TN: literal cmd at index 0, tainted var at index 1, no code-exec vector.
    let src = "set raw [gets stdin]\ninterp eval $child [list puts $raw]\n";
    assert!(
        !fires(src, D, "T105"),
        "FP-TNT-03 TN: [list puts $raw] interp eval must suppress T105; emitted {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-TNT-04 — late `--` does not protect earlier option candidates (T102).
// ---------------------------------------------------------------------------

#[test]
fn fp_tnt_04_late_terminator_does_not_protect_tainted_var() {
    // TP: `regexp $pat -- subject` — tainted $pat at index 0 (option region).
    let src = "set pat [gets stdin]\nregexp $pat -- subject\n";
    assert!(
        fires(src, D, "T102"),
        "FP-TNT-04 TP: late `--` must not protect earlier candidate; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_04_early_terminator_protects_tainted_var() {
    // TN: `regexp -- $pat subject` — `--` at index 0 terminates option scanning.
    let src = "set pat [gets stdin]\nregexp -- $pat subject\n";
    assert!(
        !fires(src, D, "T102"),
        "FP-TNT-04 TN: early `--` must protect later candidate; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_04_no_terminator_fires() {
    // TP control: `regexp $pat subject` — tainted $pat at index 0, no `--`.
    let src = "set pat [gets stdin]\nregexp $pat subject\n";
    assert!(
        fires(src, D, "T102"),
        "FP-TNT-04 TP: tainted pattern with no `--` must fire T102; emitted {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-TNT-05 — T104 honours registry network-sink arg positions.
// ---------------------------------------------------------------------------

#[test]
fn fp_tnt_05_tainted_url_fires_t104() {
    // TP: `http::geturl $url` — tainted $url at the network-address slot.
    let src = "set url [gets stdin]\nhttp::geturl $url\n";
    assert!(
        fires(src, D, "T104"),
        "FP-TNT-05 TP: tainted URL must fire T104; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_05_tainted_header_value_no_t104() {
    // TN: tainted -headers value is not the network address.
    let src = "set hdr [gets stdin]\nhttp::geturl http://example.com -headers $hdr\n";
    assert!(
        !fires(src, D, "T104"),
        "FP-TNT-05 TN: tainted header value must NOT fire T104; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_tnt_05_socket_tainted_host_and_port_fire() {
    // TP control: socket declares positions (0,1) host+port.
    let src_host = "set host [gets stdin]\nsocket $host 80\n";
    let src_port = "set port [gets stdin]\nsocket example.com $port\n";
    assert!(
        fires(src_host, D, "T104"),
        "FP-TNT-05 TP: tainted host must fire T104; emitted {:?}",
        codes(src_host, D)
    );
    assert!(
        fires(src_port, D, "T104"),
        "FP-TNT-05 TP: tainted port must fire T104; emitted {:?}",
        codes(src_port, D)
    );
}
