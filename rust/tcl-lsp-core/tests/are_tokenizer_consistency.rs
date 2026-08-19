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

//! Consistency guard tying the semantic-token regex sub-tokeniser to the
//! runtime ARE engine (`tcl-regex`), the same port Tcl uses to match.
//!
//! The sub-tokeniser in `semantic_tokens` recognises ARE constructs with a
//! hand-rolled scanner. This test uses the ARE engine as the source of truth
//! for *what is a valid ARE pattern*, then asserts the tokeniser handles every
//! such pattern cleanly: the pattern region is covered contiguously by tokens
//! with no gaps and no overlaps (the invariant LSP clients require). A
//! regression that drops a byte, overlaps two tokens, or mis-spans a construct
//! (as the POSIX-bracket bug once did) fails here.

use tcl_lsp_core::semantic_tokens::{full, legend_token_types};
use tcl_regex::Regex;
use tcl_regex::defs::REG_ADVANCED;
use tcl_registry::CommandRegistry;

/// ARE patterns spanning the construct space: literals, quantifiers (greedy /
/// non-greedy / bounded), groups (capturing / non-capturing / lookahead /
/// embedded flags), alternation, anchors, escapes, backreferences, and — the
/// case the hand-rolled scanner used to get wrong — bracket expressions with
/// POSIX / collating / equivalence sub-brackets. Kept ASCII so a UTF-16 LSP
/// column equals a byte offset on the single source line.
const ARE_CORPUS: &[&str] = &[
    "abc",
    "a+b*c?",
    "a+?b*?",
    "a{2,3}",
    "a{2,}x",
    "(ab)+",
    "(?:ab)+",
    "(?=ab)c",
    "(?!ab)c",
    "(?i)abc",
    "a|b|cd",
    "^start$",
    "\\d+\\w*\\s?",
    "\\bword\\y",
    "(a)(b)\\1\\2",
    "[a-z]+",
    "[^0-9]",
    "[]a]",
    "[[:alpha:]]+",
    "[[:digit:][:space:]]",
    "[[.hyphen.]]",
    "[[=a=]]",
    "x[[:upper:]]y[[:lower:]]z",
    "a.b.c",
    "\\(literal\\)",
    "colou?r",
    "(foo|bar)+baz",
];

fn regexp_type_indices() -> Vec<u32> {
    legend_token_types()
        .iter()
        .enumerate()
        .filter(|(_, name)| name.starts_with("regexp"))
        .map(|(i, _)| u32::try_from(i).unwrap())
        .collect()
}

/// Decode `SemanticTokens` (delta-encoded) into absolute
/// `(line, col, len, kind)` tuples.
fn decode(data: &[u32]) -> Vec<(u32, u32, u32, u32)> {
    let (mut line, mut col) = (0u32, 0u32);
    let mut out = Vec::new();
    for c in data.chunks(5) {
        let (dl, dc, len, kind) = (c[0], c[1], c[2], c[3]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len, kind));
    }
    out
}

#[test]
fn are_engine_confirms_corpus_is_valid() {
    // The corpus must be genuinely valid ARE — otherwise the tokeniser
    // consistency claim below is vacuous. The runtime engine is the authority.
    for pat in ARE_CORPUS {
        assert!(
            Regex::compile_str(pat, REG_ADVANCED).is_ok(),
            "corpus pattern is not valid ARE per the engine: {pat:?}"
        );
    }
}

#[test]
fn tokeniser_covers_valid_are_without_gaps_or_overlaps() {
    let registry = CommandRegistry::build_default();
    let regexp_kinds = regexp_type_indices();
    for pat in ARE_CORPUS {
        // `regexp {PAT} $s` — the braced literal keeps the pattern verbatim
        // (its own braces stay balanced), and the pattern occupies bytes
        // [prefix, prefix+len) on line 0.
        let prefix = "regexp {";
        let src = format!("{prefix}{pat}}} $s\n");
        let toks = decode(&full(&src, tcl_dialect::DialectProfile::by_name("tcl"), &registry).data);
        let start = u32::try_from(prefix.len()).unwrap();
        let end = start + u32::try_from(pat.len()).unwrap();

        // No two tokens overlap anywhere (global LSP invariant).
        let mut sorted = toks.clone();
        sorted.sort_by_key(|&(l, c, _, _)| (l, c));
        for w in sorted.windows(2) {
            let (l0, c0, len0, _) = w[0];
            let (l1, c1, _, _) = w[1];
            if l0 == l1 {
                assert!(
                    c1 >= c0 + len0,
                    "token overlap in {pat:?}: tokens={sorted:?}"
                );
            }
        }

        // Every byte of the pattern region is covered by some token — no byte
        // left unclassified. Each token is clamped to [start, end) so the
        // metachar-free fallback token (which spans the whole `{PAT}` word,
        // braces included) still counts, as do the content-only sub-tokens on
        // the metacharacter path.
        let mut covered = vec![false; (end - start) as usize];
        let mut regex_bytes = 0usize;
        for &(l, c, len, k) in &toks {
            if l != 0 {
                continue;
            }
            let (ts, te) = (c, c + len);
            let (cs, ce) = (ts.max(start), te.min(end));
            if cs < ce {
                for b in cs..ce {
                    covered[(b - start) as usize] = true;
                }
                if regexp_kinds.contains(&k) {
                    regex_bytes += (ce - cs) as usize;
                }
            }
        }
        assert!(
            covered.iter().all(|&b| b),
            "pattern not fully covered in {pat:?}: covered={covered:?}"
        );
        assert!(regex_bytes > 0, "no regex-family tokens covered {pat:?}");
    }
}
