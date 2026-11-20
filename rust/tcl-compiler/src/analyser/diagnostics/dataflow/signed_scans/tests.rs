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
    for source in ["proc p {data} { binary scan $data S len\n if {$len > 40000} { return big } }"] {
        assert_eq!(w156("tcl8.6", source).len(), 1, "{source}");
    }
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
    assert!(w156("f5-irules", &wrap("%ld")).is_empty());
    assert!(
        w156(
            "tcl8.4",
            "set text x\nscan $text %lx id\nif {$id == 0xffffffff} { puts max }"
        )
        .is_empty()
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
