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

use super::*;
#[test]
fn original_sslictcl_document_segment_keeps_body_config_and_crlf_channel() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Readonly DSL source parsing; no native String or execution receipt.
    let source = "set x ${a{}";
    let config = LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..file_config().nested()
    };
    let mut sink = Sink::default();
    assert!(Document::segment(source, 19, config, &mut sink).is_some());
    assert!(sink.items.is_empty());
    let mut sink = Sink::default();
    assert!(Document::segment(source, 19, file_config().nested(), &mut sink).is_none());
    assert_eq!(sink.items[0].range.start(), 19);
    assert_eq!(sink.items[0].code, DiagCode::Sslic1001);
    let mut sink = Sink::default();
    assert!(Document::segment("# c\\\r\n", 0, file_config(), &mut sink).is_none());
    assert_eq!(
        sink.items[0].message,
        "incomplete SslicTcl declaration or unclosed delimiter"
    );
    let mut sink = Sink::default();
    let file = Document::segment("\u{feff}sslictcl 1", 0, file_config(), &mut sink).unwrap();
    assert_eq!(file[0].name(), "sslictcl");
    let body =
        Document::segment("\u{feff}sslictcl 1", 11, file_config().nested(), &mut sink).unwrap();
    assert_eq!(body[0].name(), "\u{feff}sslictcl");
}
