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

use super::object_dispatch::object_handle_name;
use super::*;
use tcl_dialect::model::{Family, SurfaceLayer};

mod argument_tokens;
mod command_identity;
mod command_roles;
mod command_tokens;
mod delta;
mod emission;
mod format_subtokens;
mod irules_document;
mod legend;
mod object_dispatch;
mod regex_subtokens;
mod script_walk;
mod variable_name_roles;

use object_dispatch::Expect;

/// The plain-Tcl release these tests tokenise under. The lenient `tcl`
/// span abstains wherever its releases could differ, so a test of plain
/// Tcl names one. A named helper keeps the 140-odd call sites readable —
/// spelling the resolution out at each one buries the assertion it
/// belongs to.
fn tcl() -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
}

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// The first-paint tier reads written words and the registry only. On
/// source whose colours need no analysis it must equal the analysed
/// result, so serving it first changes nothing the user sees. The cases
/// are release-invariant; run under Tcl 8.6.
#[test]
fn coarse_tokens_equal_analysed_tokens_on_plain_source() {
    let registry = release_reg("tcl8.6");
    let sources = [
        "namespace eval ::bench {\n    variable counter 0\n}\n",
        "# a comment\nproc ::bench::step {a b} {\n    set v [expr {$a + $b}]\n    \
             set msg \"step = $v\"\n    if {$v > 10} {\n        # nested comment\n        \
             set v [expr {$v + 1}]\n    }\n    return $v\n}\n",
        "set s [string length abc] ;# inline comment\nputs -nonewline $s\n",
    ];
    for source in sources {
        assert_eq!(
            coarse(source, tcl(), &registry).data,
            full(source, tcl(), &registry).data,
            "{source}"
        );
    }
}

/// What the first-paint tier colours without any analysis: a class body's
/// members and their parameters, a parameter default, and a `switch` arm's
/// script. Release-invariant; run under Tcl 8.6.
#[test]
fn coarse_tokens_read_definition_bodies_and_case_arms_from_written_words() {
    let registry = release_reg("tcl8.6");
    let source = "oo::class create Stack {\n    method push {x {n 1}} {\n        \
                      switch -exact -- $x {\n            a { puts one }\n        }\n    }\n}\n";
    let lines: Vec<&str> = source.lines().collect();
    let (mut line, mut column) = (0usize, 0usize);
    let mut kinds = FxHashMap::default();
    for token in coarse(source, tcl(), &registry).data.chunks(5) {
        if token[0] > 0 {
            line += token[0] as usize;
            column = token[1] as usize;
        } else {
            column += token[1] as usize;
        }
        let text = &lines[line][column..column + token[2] as usize];
        kinds.entry(text).or_insert(token[3]);
    }
    for (text, kind) in [
        ("method", TokenKind::Keyword),
        ("x", TokenKind::Parameter),
        ("n", TokenKind::Parameter),
        ("1", TokenKind::Number),
        ("switch", TokenKind::Keyword),
        ("puts", TokenKind::Function),
    ] {
        assert_eq!(kinds.get(text).copied(), Some(kind as u32), "{text}");
    }
}

/// The command set of one release, so a unit built from it carries that
/// release rather than the dialect-blind standalone configuration.
fn release_reg(dialect: &str) -> CommandRegistry {
    reg().project_for_profile(
        tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
    )
}

fn expect_reg() -> CommandRegistry {
    let mut registry = reg();
    registry.load_surface(SurfaceLayer::Package("expect"));
    registry
}

fn kinds(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<u32> {
    full(src, dialect, registry)
        .data
        .chunks(5)
        .map(|c| c[3])
        .collect()
}

/// Decode the packed stream into absolute `(line, col, len)` triples.
fn decode(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<(u32, u32, u32)> {
    let st = full(src, dialect, registry);
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len) = (c[0], c[1], c[2]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len));
    }
    out
}

/// Decode the packed stream into absolute
/// `(line, col, len, kind, modifiers)` tuples.
fn decode_full(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<(u32, u32, u32, u32, u32)> {
    let st = full(src, dialect, registry);
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len, kind, mods) = (c[0], c[1], c[2], c[3], c[4]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len, kind, mods));
    }
    out
}

/// Assert no two tokens on the same line overlap (next starts at or
/// after the previous token's end) — the client "Overlapping semantic
/// tokens detected" invariant.
fn assert_non_overlapping(src: &str, registry: &CommandRegistry) {
    let toks = decode(src, tcl(), registry);
    for w in toks.windows(2) {
        let (l0, c0, len0) = w[0];
        let (l1, c1, _) = w[1];
        if l0 == l1 {
            assert!(
                c1 >= c0 + len0,
                "overlap on line {l0}: token at col {c1} starts before \
                     previous token end {} (src={src:?}, toks={toks:?})",
                c0 + len0,
            );
        }
    }
}

use Expect::{Abstain, Resolve};

/// Decode a `SemanticTokens` value directly into
/// `(line, col, len, kind, mods)` tuples.
pub(super) fn decode_semantic(st: &SemanticTokens) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len, kind, mods) = (c[0], c[1], c[2], c[3], c[4]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len, kind, mods));
    }
    out
}

/// True if any token in `src` covers exactly `text` with kind `kind`.
fn has_token_kind(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    text: &str,
    kind: TokenKind,
) -> bool {
    decode_full(src, dialect, registry)
        .iter()
        .any(|&(l, c, len, k, _)| {
            k == kind as u32
                && src
                    .lines()
                    .nth(l as usize)
                    .and_then(|line| {
                        let s = line.char_indices().nth(c as usize).map(|(i, _)| i)?;
                        let e = line
                            .char_indices()
                            .nth((c + len) as usize)
                            .map_or(line.len(), |(i, _)| i);
                        line.get(s..e)
                    })
                    .is_some_and(|got| got == text)
        })
}

#[test]
fn semantic_tokens_are_dialect_aware_via_expand_syntax() {
    // The provider re-segments
    // under the document dialect.  In `foo {*}$x`, on 8.5+ the `{*}`
    // is the expansion operator (consumed — not a highlighted word),
    // but on 8.4 it is a literal braced string `{*}`, which adds an
    // extra `string` token.  So the packed token stream is longer on
    // 8.4.
    let src = "foo {*}$x\n";
    let on_90 = full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &reg(),
    )
    .data;
    let on_84 = full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        &reg(),
    )
    .data;
    assert!(
        on_84.len() > on_90.len(),
        "8.4 keeps `{{*}}` as a highlighted string token (longer stream): \
             8.4={} 9.0={}",
        on_84.len(),
        on_90.len(),
    );
}

#[test]
fn full_returns_non_empty_data_for_simple_proc() {
    let s = full("proc foo {} {}\n", tcl(), &reg());
    // Should have at least: `proc` (keyword), `foo`
    // (function), `{}` (string), `{}` (string).
    assert!(!s.data.is_empty(), "{:?}", s.data);
    // 5 ints per token.
    assert_eq!(s.data.len() % 5, 0);
}

/// Decode the packed stream into `(line, col, len, kind)` tuples plus
/// the covered source word (ASCII sources only — byte == utf16).
fn decode_words(src: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32, u32, String)> {
    let st = full(src, tcl(), registry);
    let lines: Vec<&str> = src.split('\n').collect();
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len, kind) = (c[0], c[1], c[2], c[3]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        let word = lines
            .get(line as usize)
            .and_then(|l| l.get(col as usize..(col + len) as usize))
            .unwrap_or("")
            .to_string();
        out.push((line, col, len, kind, word));
    }
    out
}

#[test]
fn empty_source_returns_empty_data() {
    assert_eq!(full("", tcl(), &reg()).data, [] as [u32; 0]);
}

// range variant

#[test]
fn range_filters_tokens_outside_window() {
    // Three commands on three lines.  Range covers only
    // line 1 — the line-0 and line-2 tokens should drop.
    let src = "set a 1\nset b 2\nset c 3\n";
    let full_data = full(src, tcl(), &reg());
    let line1_only = range(
        src,
        tcl(),
        crate::definition::LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 10,
        },
        &reg(),
    );
    // Each tcl line emits at least one classified token.
    // The range result must be strictly smaller than the
    // full result.
    assert!(line1_only.data.len() < full_data.data.len());
    assert!(line1_only.data.len().is_multiple_of(5));
    assert!(!line1_only.data.is_empty(), "{:?}", line1_only.data);
}

#[test]
fn range_keeps_entire_document_when_range_covers_it() {
    let src = "proc foo {} { puts hi }\n";
    let full_data = full(src, tcl(), &reg());
    let wide = range(
        src,
        tcl(),
        crate::definition::LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 99,
            end_character: 0,
        },
        &reg(),
    );
    assert_eq!(wide.data, full_data.data);
}

#[test]
fn range_excludes_token_at_exact_end_position() {
    // LSP ranges are
    // half-open [start, end), so a token starting exactly
    // at `end` is OUTSIDE the range.
    let src = "set a 1\nset b 2\n";
    // Range whose end exactly coincides with line 1, col 0
    // (the `set` of the second command).  That token should
    // not appear in the range result.
    let r = range(
        src,
        tcl(),
        crate::definition::LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 1,
            end_character: 0,
        },
        &reg(),
    );
    // The full document has at least one line-1 token at col
    // 0 (the `set` of `set b 2`).  The half-open range must
    // exclude it; the range data must therefore be strictly
    // shorter than the full data.
    let full_data = full(src, tcl(), &reg());
    assert!(
        r.data.len() < full_data.data.len(),
        "range data {} should drop the line-1 token; full data {}",
        r.data.len(),
        full_data.data.len(),
    );
}
