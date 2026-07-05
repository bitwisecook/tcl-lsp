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

//! PARSER-CONVERGENCE PROBE (dev tool).
use tcl_lexer::{Lexer, SourceMap, TokenType};
fn dump(s: &str) {
    let sm = SourceMap::new(s);
    println!("{s:?}:");
    for t in Lexer::new(s).tokenise_all().unwrap() {
        if t.kind == TokenType::Eof {
            continue;
        }
        let content_start = t.span.start() as usize + t.content_offset as usize;
        let content = &s.as_bytes()[content_start..t.span.end() as usize];
        println!(
            "  {:7} span[{},{}) coff={} inq={} text={:?} content={:?}",
            t.kind.name(),
            t.span.start(),
            t.span.end(),
            t.content_offset,
            t.in_quote,
            sm.text(t.span),
            String::from_utf8_lossy(content)
        );
    }
}
fn main() {
    for s in [
        "{}",
        "llength {}",
        "\"hi there\"",
        "{a b}",
        "set out {}",
        "\"hi $x there\"",
        "set x \"a[foo]b\"",
        "puts $x",
        "a$b c",
        "foo\\nbar",
        "{*}$x",
        "[foo]",
        "\"\"",
        "set x {a\\nb}",
        "a;b",
        "# comment\nfoo",
        "\"$x foo\"",
        "\"[foo]bar\"",
        "$a(b)",
        "$a($i)",
        "puts \"\"",
    ] {
        dump(s);
    }
}
