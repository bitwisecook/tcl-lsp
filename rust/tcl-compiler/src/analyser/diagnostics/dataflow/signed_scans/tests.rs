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

fn irule5008(dialect: &str, source: &str) -> Vec<Diagnostic> {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::Irule5008)
        .collect()
}

#[test]
fn byte_compared_with_unsigned_constant_is_reported_on_the_variable() {
    let source = "when CLIENT_DATA {\n  binary scan [TCP::payload] c type\n  if {$type == 0x81} { log local0. hit }\n}";
    let found = irule5008("f5-irules", source);
    let [diagnostic] = found.as_slice() else {
        panic!("{found:?}");
    };
    assert_eq!(&source[diagnostic.span.as_range()], "$type");
    assert!(diagnostic.message.contains("-128..127"), "{}", diagnostic.message);
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
        assert!(irule5008("f5-irules", source).is_empty(), "{source}");
    }
}

#[test]
fn later_fields_shorts_and_relational_tests_are_tracked() {
    let source = "when CLIENT_DATA {\n  binary scan [TCP::payload] xcS flags len\n  if {$len > 40000 || 0x80 <= $flags} { log local0. big }\n}";
    let found = irule5008("f5-irules", source);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].message.contains("always false"));
}

#[test]
fn scan_hex_wraps_to_int32() {
    let source = "when HTTP_REQUEST {\n  scan [HTTP::header value X-Id] %x id\n  if {$id == 0xffffffff} { log local0. max }\n}";
    let found = irule5008("f5-irules", source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("-1"));
}

#[test]
fn only_irules_reports() {
    let source = "binary scan $data c type\nif {$type == 0x81} { puts hit }";
    assert!(irule5008("tcl8.6", source).is_empty());
}
