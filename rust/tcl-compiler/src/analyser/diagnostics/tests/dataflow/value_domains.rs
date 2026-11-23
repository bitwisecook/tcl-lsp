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

//! W124 malformed IP literals and W233 division by zero.

use super::*;

#[test]
fn w124_distinguishes_times_and_mac_addresses_from_invalid_ipv6() {
    let mut analyser = Analyser::new();
    for other in ["12:30:45", "00:11:22:aa:bb:cc"] {
        let result = analyser.analyse(&format!("proc inspect {{}} {{ set value {other} }}"), "tcl");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W124),
            "{other}"
        );
    }
    let result = analyser.analyse("proc inspect {} { set value 2001:db8:abcd:1 }", "tcl");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W124)
    );
}

#[test]
fn ipv6_candidate_scanner_extracts_runs() {
    let c = super::super::super::find_ipv6_candidates("addr fe80::1 end");
    assert_eq!(c, vec!["fe80::1"]);
    // A bare hextet pair (only one colon → <2 groups) is not a
    // candidate; a full address is.
    assert_eq!(
        super::super::super::find_ipv6_candidates("ab:cd"),
        [] as [&str; 0]
    );
    assert_eq!(
        super::super::super::find_ipv6_candidates("2001:db8::8a2e:370:7334"),
        vec!["2001:db8::8a2e:370:7334"]
    );
}

#[test]
fn analyse_w124_ipv4_octet_overflow() {
    // ``proc foo {} { set ip 192.168.1.999 }`` — 999 > 255,
    // not a valid IP.  W124 fires at the assignment.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 192.168.1.999 }", "tcl");
    let w124s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W124)
        .collect();
    assert!(
        !w124s.is_empty(),
        "W124 expected for IPv4 octet > 255; got {:?}",
        r.diagnostics,
    );
    assert!(w124s[0].message.contains("999"));
    assert!(w124s[0].message.contains("exceeds 255"));
    assert_eq!(w124s[0].severity, Severity::Error);
}

#[test]
fn analyse_no_w124_for_valid_ipv4() {
    // ``proc foo {} { set ip 192.168.1.1 }`` — valid IP.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 192.168.1.1 }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W124),
        "W124 must not fire on valid IPv4; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w124_ipv4_leading_zero_warning() {
    // ``proc foo {} { set ip 192.168.01.1 }`` — leading
    // zero on octet 3; might be octal in some contexts.
    // Severity is Warning.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 192.168.01.1 }", "tcl");
    let w124s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W124)
        .collect();
    assert!(
        !w124s.is_empty(),
        "W124 expected for IPv4 leading-zero octet; got {:?}",
        r.diagnostics,
    );
    assert_eq!(w124s[0].severity, Severity::Warning);
    assert!(w124s[0].message.contains("leading zero"));
}

#[test]
fn analyse_no_w124_for_oid_chain() {
    // FP-STY-06: an LDAP PEN OID (`1.3.6.1.4.1.4203.1.11.3`) is a
    // hierarchical dotted chain, not IPv4 — the embedded `4203.1.11.3`
    // slice must NOT fire W124.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set oid 1.3.6.1.4.1.4203.1.11.3 }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W124),
        "W124 must not fire on an OID dotted chain; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w124_real_ipv4_shaped_still_fires() {
    // TP control: a genuine four-component dotted quad with an
    // out-of-range octet (not part of a longer chain) still fires.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set ip 10.0.0.300 }", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W124),
        "W124 must fire on a genuine over-255 IPv4 quad; got {:?}",
        r.diagnostics,
    );
}

fn w233_codes(src: &str) -> usize {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W233)
        .count()
}

#[test]
fn w233_divide_by_zero_literal_and_const_var() {
    // Each case verified against tclsh 8.4–9.0 to raise "divide by zero".
    assert_eq!(w233_codes("proc f {} { return [expr {1 / 0}] }"), 1);
    assert_eq!(w233_codes("proc f {} { return [expr {10 % 0}] }"), 1);
    assert_eq!(
        w233_codes("proc f {} { set d 0\n return [expr {10 / $d}] }"),
        1
    );
}

#[test]
fn w233_silent_on_nonzero_unknown_and_guarded() {
    // Non-zero const + unknown divisor never fire.
    assert_eq!(
        w233_codes("proc f {} { set d 3\n return [expr {10 / $d}] }"),
        0
    );
    assert_eq!(w233_codes("proc f {n} { return [expr {10 / $n}] }"), 0);
    // Short-circuit / dead-arm guards make the division unreachable
    // (tclsh: these expressions do *not* raise — the RHS is skipped).
    assert_eq!(w233_codes("proc f {} { return [expr {0 && 1/0}] }"), 0);
    assert_eq!(w233_codes("proc f {} { return [expr {0 ? 1/0 : 7}] }"), 0);
    assert_eq!(w233_codes("proc f {c} { return [expr {$c && 1/0}] }"), 0);
    assert_eq!(w233_codes("proc f {} { return [expr {+0 && 1/0}] }"), 0);
    // Constant-truthy guard forces the arm — fires. Each verified against
    // tclsh 8.4–9.0 (`-1`, `1.0`, `!0` are truthy, so `1/0` is reached
    // and raises "divide by zero").
    assert_eq!(w233_codes("proc f {} { return [expr {1.0 && 1/0}] }"), 1);
    assert_eq!(w233_codes("proc f {} { return [expr {-1 && 1/0}] }"), 1);
    assert_eq!(w233_codes("proc f {} { return [expr {!0 && 1/0}] }"), 1);
}

#[test]
fn w233_silent_on_float_division() {
    // Float division by zero yields Inf (verified against tclsh 8.4–9.0:
    // `1.0/0.0` and `1/0.0` are *not* errors), so the integer-only
    // divide-by-zero check must not fire. A float-literal divisor never
    // narrows to the integer point `[0, 0]`.
    assert_eq!(w233_codes("proc f {} { return [expr {1.0 / 0.0}] }"), 0);
    assert_eq!(w233_codes("proc f {} { return [expr {1 / 0.0}] }"), 0);
}
