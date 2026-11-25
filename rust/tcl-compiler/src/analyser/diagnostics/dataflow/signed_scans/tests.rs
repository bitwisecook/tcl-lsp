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

use crate::analyser::state::Analyser;
use crate::analyser::types::Diagnostic;
use tcl_core_types::DiagCode;

fn w156(dialect: &str, source: &str) -> Vec<Diagnostic> {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W156)
        .collect()
}

#[test]
fn byte_compared_with_unsigned_constant_is_reported_on_the_variable() {
    let source = "when CLIENT_DATA {\n  binary scan [TCP::payload] c type\n  if {$type == 0x81} { log local0. hit }\n}";
    let found = w156("f5-irules", source);
    let [diagnostic] = found.as_slice() else {
        panic!("{found:?}");
    };
    assert_eq!(&source[diagnostic.span.as_range()], "$type");
    assert!(
        diagnostic.message.contains("-128..127"),
        "{}",
        diagnostic.message
    );
    assert!(diagnostic.message.contains("($type & 0xff) == 0x81"));
    assert!(diagnostic.message.contains("-127"));
}

#[test]
fn masked_signed_in_range_and_unsigned_reads_are_quiet() {
    for source in [
        "when CLIENT_DATA { binary scan [TCP::payload] c type\n if {($type & 0xff) == 0x81} { log local0. hit } }",
        "when CLIENT_DATA { binary scan [TCP::payload] c type\n if {$type == -127} { log local0. hit } }",
        "when CLIENT_DATA { binary scan [TCP::payload] c type\n set type [expr {$type & 0xff}]\n if {$type == 0x81} { log local0. hit } }",
        "when CLIENT_DATA { binary scan [TCP::payload] H2 type\n if {$type == 0x81} { log local0. hit } }",
        "when CLIENT_DATA { binary scan [TCP::payload] c2 type\n if {$type == 200} { log local0. hit } }",
    ] {
        assert!(w156("f5-irules", source).is_empty(), "{source}");
    }
}

#[test]
fn later_fields_shorts_and_relational_tests_are_tracked() {
    let source = "when CLIENT_DATA {\n  binary scan [TCP::payload] xcS flags len\n  if {$len > 40000 || 0x80 <= $flags} { log local0. big }\n}";
    let found = w156("f5-irules", source);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].message.contains("always false"));
}

#[test]
fn scan_hex_wraps_to_int32() {
    let source = "when HTTP_REQUEST {\n  scan [HTTP::header value X-Id] %x id\n  if {$id == 0xffffffff} { log local0. max }\n}";
    let found = w156("f5-irules", source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("-1"));
}

/// The release matrix measured in `evidence/tcl/scan-integer-signedness.md`:
/// `binary scan` integer fields are signed everywhere and take a `u` suffix
/// from 8.5 and in Jim; `scan` truncates to signed 32 bits on 8.4 and 9.x.
#[test]
fn every_dialect_follows_its_measured_signedness() {
    let byte = "set data x\nbinary scan $data c type\nif {$type == 0x81} { puts hit }";
    let hex = "set text x\nscan $text %x id\nif {$id == 0xffffffff} { puts max }";
    let unsigned = "set text x\nscan $text %u id\nif {$id == 4294967295} { puts max }";
    let rows: &[(&str, &str, Option<&str>)] = &[
        (
            "tcl8.4",
            byte,
            Some("Mask it as `($type & 0xff) == 0x81`, or compare with the signed value -127."),
        ),
        ("tcl8.5", byte, Some("Read it unsigned with `cu`")),
        ("tcl8.6", byte, Some("Read it unsigned with `cu`")),
        ("tcl9.0", byte, Some("Read it unsigned with `cu`")),
        ("jim", byte, Some("Read it unsigned with `cu`")),
        (
            "f5-irules",
            "when CLIENT_DATA { binary scan [TCP::payload] c type\nif {$type == 0x81} { log local0. hit } }",
            Some("Mask it as"),
        ),
        ("tcl8.4", hex, Some("Read it at full width with `%lx`")),
        ("tcl8.5", hex, None),
        ("tcl8.6", hex, None),
        ("tcl9.0", hex, Some("Read it at full width with `%lx`")),
        ("jim", hex, None),
        ("tcl8.4", unsigned, Some("Read it at full width with `%lu`")),
        ("tcl8.6", unsigned, None),
        ("tcl9.0", unsigned, None),
    ];
    for &(dialect, source, expected) in rows {
        let found = w156(dialect, source);
        match expected {
            None => assert!(found.is_empty(), "{dialect}: {source}: {found:?}"),
            Some(text) => {
                let [diagnostic] = found.as_slice() else {
                    panic!("{dialect}: {source}: {found:?}");
                };
                assert!(
                    diagnostic.message.contains(text),
                    "{dialect}: {}",
                    diagnostic.message
                );
            }
        }
    }
}

#[test]
fn unsigned_and_wide_reads_are_quiet() {
    for (dialect, source) in [
        (
            "tcl8.6",
            "set data x\nbinary scan $data cu type\nif {$type == 0x81} { puts hit }",
        ),
        (
            "tcl9.0",
            "set text x\nscan $text %lx id\nif {$id == 0xffffffff} { puts max }",
        ),
        (
            "tcl8.4",
            "set text x\nscan $text %ld id\nif {$id == 4294967295} { puts max }",
        ),
    ] {
        assert!(w156(dialect, source).is_empty(), "{dialect}: {source}");
    }
}

#[test]
fn a_short_size_becomes_the_long_size() {
    let found = w156(
        "tcl9.0",
        "set text x\nscan $text %hx id\nif {$id == 0xffffffff} { puts max }",
    );
    assert!(found[0].message.contains("`%lx`"), "{}", found[0].message);
}

#[test]
fn procedures_are_followed() {
    let source = "proc p {data} { binary scan $data S len\n if {$len > 40000} { return big } }";
    assert_eq!(w156("tcl8.6", source).len(), 1, "{source}");
}

#[test]
fn straight_line_procedure_reads_follow_the_scan() {
    for (source, expected) in [
        (
            "proc p {data} { binary scan $data S len\n if {$len > 40000} { return big } }",
            1,
        ),
        (
            "namespace eval n { proc p {data} { binary scan $data c t\n if {$t != 0x81} { return } } }",
            1,
        ),
        (
            "proc p {data} { binary scan $data S len\n set len 5\n if {$len > 40000} { return } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data S len\n incr len\n if {$len > 40000} { return } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data S len\n foo\n if {$len > 40000} { return } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data S len\n mangle len\n if {$len > 40000} { return } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data cu t\n if {$t == 200} { return } }",
            0,
        ),
    ] {
        assert_eq!(w156("tcl8.6", source).len(), expected, "{source}");
    }
}

/// TMM truncates `%lx` as well as `%x`, where stock 8.4 reads `%lx` at full
/// width (`evidence/tcl/bigip-tmm-signed-scan-and-format.md`), so an iRule
/// is told to mask and never to add an `l` size.
#[test]
fn irules_follow_tmm_scan_truncation() {
    let wrap = |format: &str| {
        format!(
            "when HTTP_REQUEST {{ scan [HTTP::header value X-Id] {format} id\n if {{$id == 0xffffffff}} {{ log local0. max }} }}"
        )
    };
    for format in ["%x", "%lx"] {
        let found = w156("f5-irules", &wrap(format));
        let [diagnostic] = found.as_slice() else {
            panic!("{format}: {found:?}");
        };
        assert!(
            diagnostic
                .message
                .contains("Mask it as `($id & 0xffffffff) == 0xffffffff`"),
            "{format}: {}",
            diagnostic.message
        );
        assert!(
            !diagnostic.message.contains("full width"),
            "{}",
            diagnostic.message
        );
    }
    assert_eq!(
        w156("f5-irules", &wrap("%ld")),
        [] as [crate::analyser::Diagnostic; 0]
    );
    assert_eq!(
        w156(
            "tcl8.4",
            "set text x\nscan $text %lx id\nif {$id == 0xffffffff} { puts max }"
        ),
        [] as [crate::analyser::Diagnostic; 0]
    );
}

/// TMM loads `binary scan … cu` and then fails at run time; the `u`-suffix
/// version gate already reports it in an iRule.
#[test]
fn irules_report_the_unsigned_suffix() {
    let source = "when CLIENT_DATA { binary scan [TCP::payload] cu type\n if {$type == 0x81} { log local0. hit } }";
    let codes: Vec<_> = Analyser::new()
        .analyse(source, "f5-irules")
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect();
    assert!(codes.contains(&DiagCode::W200), "{codes:?}");
}

fn located(dialect: &str, source: &str) -> Vec<(String, String)> {
    w156(dialect, source)
        .into_iter()
        .map(|d| (source[d.span.as_range()].to_owned(), d.message))
        .collect()
}

/// Byte composition with signed parts, measured in
/// `evidence/tcl/signed-scan-downstream-consumers.md`.
#[test]
fn byte_compositions_report_each_signed_part() {
    let source = "set data x\nbinary scan $data cc hi lo\nset port [expr {($hi << 8) | $lo}]";
    let found = located("tcl8.6", source);
    let spans: Vec<&str> = found.iter().map(|(span, _)| span.as_str()).collect();
    assert_eq!(spans, ["$hi", "$lo"], "{found:?}");
    assert!(found[1].1.contains("sign-extends"), "{}", found[1].1);
    assert!(found[1].1.contains("($lo & 0xff)"), "{}", found[1].1);
    let sum = located(
        "tcl8.4",
        "set data x\nbinary scan $data cc hi lo\nset n [expr {$hi * 256 + $lo}]",
    );
    assert_eq!(sum.len(), 2, "{sum:?}");
    for quiet in [
        "set data x\nbinary scan $data cc hi lo\nset port [expr {(($hi & 0xff) << 8) | ($lo & 0xff)}]",
        "set data x\nbinary scan $data cucu hi lo\nset port [expr {($hi << 8) | $lo}]",
        "set data x\nbinary scan $data cc hi lo\nset n [expr {$hi + $lo}]",
    ] {
        assert!(w156("tcl8.6", quiet).is_empty(), "{quiet}");
    }
}

#[test]
fn format_conversions_that_render_a_negative_value_wrongly() {
    let rows: &[(&str, &[&str])] = &[
        ("format %02x $b", &["$b"]),
        ("format %u $b", &["$b"]),
        ("format %c $b", &["$b"]),
        ("format %d.%d.%d.%d 192 $b 1 2", &["$b"]),
        ("format %*x 4 $b", &["$b"]),
        ("format {%d %d} $b $b", &[]),
        ("format %02x [expr {$b & 0xff}]", &[]),
        ("format %d $b", &[]),
        ("format {%1$x} $b", &[]),
    ];
    for &(call, expected) in rows {
        let source = format!("set data x\nbinary scan $data c b\nset out [{call}]");
        let spans: Vec<String> = located("tcl8.6", &source)
            .into_iter()
            .map(|(span, _)| span)
            .collect();
        assert_eq!(spans, expected, "{call}");
    }
}

#[test]
fn irules_consumers_in_event_bodies() {
    let source = "when CLIENT_DATA {\n  binary scan [TCP::payload] cccc a b c d\n  log local0. [format %d.%d.%d.%d $a $b $c $d]\n  log local0. [format %02x $a]\n}";
    let found = located("f5-irules", source);
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(
        found.iter().any(|(_, m)| m.contains("192.-64.-1.1")),
        "{found:?}"
    );
}

/// Arguments the registry declares non-negative (measured on BIG-IP 21.1 or
/// stock Tcl) and index arguments.
#[test]
fn signed_values_reaching_non_negative_arguments() {
    let irule = |body: &str| {
        format!("when CLIENT_DATA {{\n  binary scan [TCP::payload] cS b len\n  {body}\n}}")
    };
    let rows: &[(&str, &[&str])] = &[
        ("TCP::collect $len", &["$len"]),
        ("TCP::release $len", &["$len"]),
        ("TCP::payload $b $len", &["$b", "$len"]),
        ("TCP::payload replace $b 1 x", &["$b"]),
        ("TCP::payload replace 0 $len x", &[]),
        ("SSL::collect $len", &["$len"]),
        ("SSL::payload $b $len", &["$b", "$len"]),
        ("SSL::payload replace $b 1 x", &["$b"]),
        ("set n [SSL::payload length]", &[]),
        ("node 10.0.0.1 $len", &["$len"]),
        ("pool p member 10.0.0.1 $len", &["$len"]),
        ("TCP::collect [expr {$len & 0xffff}]", &[]),
        ("log local0. $len", &[]),
    ];
    for &(body, expected) in rows {
        let source = irule(body);
        let spans: Vec<String> = located("f5-irules", &source)
            .into_iter()
            .map(|(span, _)| span)
            .collect();
        assert_eq!(spans, expected, "{body}");
    }
    let message = &located("f5-irules", &irule("TCP::collect $len"))[0].1;
    assert!(
        message.contains("`TCP::collect` takes a non-negative value"),
        "{message}"
    );
    assert!(message.contains("[expr {$len & 0xffff}]"), "{message}");
    let replace = &located("f5-irules", &irule("TCP::payload replace $b 1 x"))[0].1;
    assert!(replace.contains("`TCP::payload replace`"), "{replace}");
    let port = &located("f5-irules", &irule("node 10.0.0.1 $len"))[0].1;
    assert!(port.contains("port (0..65535)"), "{port}");
}

#[test]
fn signed_values_used_as_indices_or_counts() {
    let stock =
        |body: &str| format!("set data x\nset l {{a b c}}\nbinary scan $data c b\nset r [{body}]");
    let rows: &[(&str, &[&str])] = &[
        ("lindex $l $b", &["$b"]),
        ("string index abc $b", &["$b"]),
        ("string repeat x $b", &["$b"]),
        ("string repeat $b 2", &[]),
        ("lindex $l [expr {$b & 0xff}]", &[]),
    ];
    for &(body, expected) in rows {
        let spans: Vec<String> = located("tcl8.6", &stock(body))
            .into_iter()
            .map(|(span, _)| span)
            .collect();
        assert_eq!(spans, expected, "{body}");
    }
}

/// An address built by interpolation renders a negative octet, which TMM's
/// `IP::addr` rejects (`evidence/tcl/bigip-tmm-signed-scan-and-format.md`).
#[test]
fn interpolated_addresses_report_signed_octets() {
    let source = "when CLIENT_DATA {\n  binary scan [TCP::payload] cccc a b c d\n  if {[IP::addr \"$a.$b.$c.$d\" equals 10.0.0.1]} { log local0. hit }\n  log local0. \"192.168.$c.$d\"\n}";
    let found = located("f5-irules", source);
    let spans: Vec<&str> = found.iter().map(|(span, _)| span.as_str()).collect();
    assert_eq!(spans, ["$a", "$b", "$c", "$d", "$c", "$d"], "{found:?}");
    let quiet = "when CLIENT_DATA {\n  binary scan [TCP::payload] cccc a b c d\n  log local0. \"$a.$b\"\n  log local0. \"v=$a.$b.$c.$d.x\"\n}";
    assert_eq!(
        w156("f5-irules", quiet),
        [] as [crate::analyser::Diagnostic; 0]
    );
}

/// In a procedure body, a command after one the module cannot prove stays
/// an unlowered call, and so do the bodies it holds; W156 reads their
/// braced expressions as the commands they normally are, across blocks.
#[test]
fn unlowered_procedure_code_is_followed() {
    let rows: &[(&str, usize)] = &[
        (
            "proc p {data} { binary scan $data c t\n expr {$t == 200} }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n set ok [expr {$t == 200}] }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n while {$t == 200} { break } }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n puts $t\n if {$t == 200} { return } }",
            1,
        ),
        (
            "proc p {data} { binary scan $data cc hi lo\n return [expr {($hi << 8) | $lo}] }",
            2,
        ),
        (
            "proc p {data} { binary scan $data c t\n return [format %02x $t] }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n if {$data ne {}} { if {$t == 200} { return } } }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n foreach x {1 2} { if {$t == 200} { return } } }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n if {$data ne {}} { while {$t > 127} { break } } }",
            1,
        ),
        (
            "proc p {data} { binary scan $data c t\n upvar 1 t x\n if {$t == 200} { return } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data c t\n foo\n if {$t == 200} { return } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data c t\n if {$data ne {}} { set t 0\n if {$t == 200} { return } } }",
            0,
        ),
        (
            "proc p {data} { binary scan $data c t\n uplevel {incr t}\n if {$t == 200} { return } }",
            0,
        ),
    ];
    for &(source, expected) in rows {
        assert_eq!(w156("tcl8.6", source).len(), expected, "{source}");
    }
}

#[test]
fn reads_in_later_blocks_follow_the_dominating_scan() {
    let rows: &[(&str, usize)] = &[
        (
            "set data x\nbinary scan $data c t\nwhile {$t == 200} { break }",
            1,
        ),
        (
            "set data x\nbinary scan $data c t\nif {$data ne {}} { if {$t == 200} { puts x } }",
            1,
        ),
        (
            "set data x\nbinary scan $data c t\nwhile {$t == 200} { set t 0 }",
            0,
        ),
    ];
    for &(source, expected) in rows {
        assert_eq!(w156("tcl8.6", source).len(), expected, "{source}");
    }
}

/// A comparison whose outcome is the same over the whole signed range,
/// including at its bound: `$b > 127` is never true for a signed byte.
#[test]
fn comparisons_at_the_signed_bound_are_constant() {
    let source =
        |test: &str| format!("set data x\nbinary scan $data c b\nif {{{test}}} {{ puts x }}");
    for (test, expected) in [
        ("$b > 127", Some("false")),
        ("$b >= 128", Some("false")),
        ("$b <= 127", Some("true")),
        ("127 < $b", Some("false")),
        ("$b > 126", None),
        ("$b < 0", None),
        ("$b == -127", None),
    ] {
        let found = w156("tcl8.6", &source(test));
        match expected {
            None => assert!(found.is_empty(), "{test}: {found:?}"),
            Some(outcome) => assert!(
                found.len() == 1 && found[0].message.contains(&format!("always {outcome}")),
                "{test}: {found:?}"
            ),
        }
    }
}
