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

//! `segmentation-drift` — the command / word boundary drift gate.
//!
//! Issue #1786 counted the answers to "where does this command end and
//! where does each word begin" and found **four** implementations of it:
//! `tcl-compiler`'s CST builder, `runtime/rust`'s `parse_script`, the
//! compiler segmenter, and `tcl_lexer::structural_index`'s byte-scanned
//! `command_boundaries`. They disagreed measurably — on `{*}` welded to a
//! close-brace, and on where a nested `[a [b {*}$c]]` ends. Each was
//! written by someone who needed a boundary in a hurry and did not know a
//! shared one existed.
//!
//! `docs/design/contracts/shared-utility-contracts-rust.md` now names two
//! surfaces for this, and this gate is the drift check on both:
//!
//! * **command / word segmentation** — `tcl_lexer::script::group_commands`
//!   over a dialect-configured token stream, with the compiler's
//!   `segment_commands` / CST builder as its consumers. Every question
//!   about where a *word* starts is this owner's.
//! * **script completeness / reparse windows** —
//!   `tcl_lexer::script_is_complete`, `command_boundaries` and
//!   `reparse_window`: the `Tcl_CommandComplete` port and the byte-scanned
//!   split points an incremental reparser snaps to, deliberately
//!   dialect-blind and cross-checked against the owner over the corpora by
//!   `tcl-lexer`'s `differential_boundaries`.
//!
//! # What it flags
//!
//! 1. **A private command-terminator scan** — a byte or `char` match
//!    pattern that alternates `\n` with `;` (`b'\n' | b';'`, `'\n' | ';'`,
//!    either order). Those two bytes together are Tcl's *command
//!    terminator set*, and a loop that tests for them is splitting a script
//!    into commands by hand. It is the exact shape of all four
//!    implementations #1786 collapsed, and of the three private splitters
//!    still live in `tcl-lsp-core` and the optimiser (each waived in place,
//!    with what it re-derives named).
//!
//! 2. **A private word-start state machine** — the two-variant
//!    `TokenType::Sep | TokenType::Eol` alternation tested against a
//!    *previous-token* binding. "A content token after a `Sep` or an `Eol`
//!    starts a new word" is `group_commands`' word rule verbatim; a
//!    consumer carrying its own `prev_type` across a token loop has
//!    reimplemented it. A `Sep`/`Eol` set that also lists `Eof` or
//!    `Comment`, or one with no `prev` binding in sight, is trivia
//!    *skipping* — legitimate, and not flagged.
//!
//! 3. **A missing cross-check** — the corpus differential that keeps the
//!    dialect-blind scanner honest against the owner must exist and must
//!    still declare its named tests. Following `number-drift`'s precedent,
//!    this is structural wiring evidence, not a semantic proof: the Rust
//!    tests execute the actual corpora.
//!
//! # Known blind spots
//!
//! A grouper that matches on token kinds *arm by arm* — `TokenType::Sep =>
//! flush_word(…)`, `TokenType::Eol => push_command(…)` — is not flagged,
//! because a `match tok.kind` with one arm per kind is also how every
//! legitimate token consumer (a formatter, a highlighter, a minifier) is
//! written. `runtime/rust/src/parse.rs` is the live example, and it is the
//! consumer PR #1818 folds onto the owner; the gate scans `rust/` only, as
//! `dialect-drift` does.
//!
//! Escapes: the owners themselves are exempt (see [`SANCTIONED_FILES`]);
//! test modules, `tests/` trees, examples and benches are skipped; and a
//! reviewed site carries `// segmentation-drift-ok: <reason>` on the
//! flagged line or in the comment block directly above it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The files allowed to derive a command or word boundary from raw bytes
/// or from a `Sep`/`Eol` state machine: the tokeniser that defines the
/// terminator grammar, the boundary owner, the two registered
/// `tcl-lexer` scanners, and the compiler's segmentation owners — i.e.
/// exactly the source paths the two owner-manifest rows name.
const SANCTIONED_FILES: &[&str] = &[
    // The tokeniser: `\n` / `;` *are* its grammar.
    "rust/tcl-lexer/src/lexer.rs",
    // The boundary owner (#1786).
    "rust/tcl-lexer/src/script.rs",
    // The registered `Tcl_CommandComplete` port + reparse split points.
    "rust/tcl-lexer/src/structural_index.rs",
    // The registered word-span / delimiter-close scanners the two above
    // and the segmenter all call.
    "rust/tcl-lexer/src/ranges.rs",
    "rust/tcl-lexer/src/word_parts.rs",
    // The compiler's half of the segmentation row.
    "rust/tcl-compiler/src/segmenter.rs",
    "rust/tcl-compiler/src/parsing/syntax/build.rs",
    "rust/tcl-compiler/src/parsing/syntax/segment.rs",
    // This gate names the banned spellings in its own source.
    "rust/xtask/src/segmentation_drift.rs",
];

/// The corpus differential rule 3 requires, and the tests it must declare.
const CROSS_CHECK_FILE: &str = "rust/tcl-lexer/tests/differential_boundaries.rs";
const CROSS_CHECK_TESTS: &[&str] = &[
    "fn scanner_agrees_with_owner_on_edge_cases",
    "fn scanner_agrees_with_owner_over_corpora",
    "fn scanner_agrees_with_owner_over_corpora_deep",
    "fn dialect_divergences_are_pinned",
];
/// Both engines must actually appear in the cross-check — a differential
/// that stopped calling one of them proves nothing.
const CROSS_CHECK_ENGINES: &[&str] = &["command_boundaries(", "group_commands("];

const WAIVER: &str = "segmentation-drift-ok:";

/// The command-terminator set, in every spelling a match pattern uses.
const TERMINATOR_NEEDLES: &[&str] = &[
    "b'\\n' | b';'",
    "b';' | b'\\n'",
    "'\\n' | ';'",
    "';' | '\\n'",
];

const TERMINATOR_WHY: &str = "scans for the Tcl command-terminator set (`\\n` / `;`) by hand — \
     that is a private top-level command splitter (#1786). Take the \
     boundaries from tcl_lexer::script::group_commands, or, for a reparse \
     split point, from tcl_lexer::command_boundaries";

/// The word-start rule, in both orders. `tcl_lexer::` prefixes are
/// normalised away before matching.
const WORD_START_NEEDLES: &[&str] = &[
    "TokenType::Sep | TokenType::Eol",
    "TokenType::Eol | TokenType::Sep",
];

const WORD_START_WHY: &str = "carries a previous-token kind across a token loop to decide where a word starts — \
     that is group_commands' word rule reimplemented. Group once through \
     tcl_lexer::script::group_commands (or the compiler's segment_commands) and read \
     WordSpan";

/// How many lines after (and before, for the `prev` binding) a needle may
/// be split across by rustfmt.
const LOOKAHEAD: usize = 8;
const LOOKBEHIND: usize = 4;

/// Scan the workspace; exit non-zero listing every offending site.
/// `check` is accepted for CLI symmetry with the other gates — the lint
/// never rewrites anything, so both modes verify.
pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    files.sort();

    let mut report = String::new();
    let mut hits = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_exempt_path(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, snippet, why) in scan(&text) {
            hits += 1;
            let _ = writeln!(report, "  {rel}:{line_no}: {snippet}\n      -> {why}");
        }
    }

    for problem in cross_check_problems(&root) {
        hits += 1;
        let _ = writeln!(report, "  {CROSS_CHECK_FILE}: {problem}");
    }

    if hits == 0 {
        println!(
            "segmentation-drift: OK (no private command-terminator scan or word-start state \
             machine outside the boundary owners; the owner/scanner corpus differential is wired)"
        );
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "segmentation-drift: {hits} site(s) derive Tcl command or word boundaries outside the \
         owner. Where a command ends and where a word begins is answered once, by \
         tcl_lexer::script::group_commands over a dialect-configured token stream \
         (shared-utility-contracts-rust.md, issue #1786); a reparse split point comes from \
         tcl_lexer::command_boundaries. If a site genuinely is not segmenting Tcl script text, \
         mark it `// {WAIVER} <reason>`:\n{report}"
    );
    ExitCode::FAILURE
}

/// Rule 3: the corpus differential exists and still names both engines and
/// its tiers.
fn cross_check_problems(root: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join(CROSS_CHECK_FILE)) else {
        return vec![
            "the owner/scanner boundary differential is missing — `command_boundaries` is \
             registered as a separate, dialect-blind surface precisely because this \
             cross-check pins it to the owner"
                .to_owned(),
        ];
    };
    let mut problems = Vec::new();
    for needed in CROSS_CHECK_TESTS {
        if !text.contains(needed) {
            problems.push(format!("the cross-check no longer declares `{needed}`"));
        }
    }
    for engine in CROSS_CHECK_ENGINES {
        if !text.contains(engine) {
            problems.push(format!(
                "the cross-check no longer calls `{engine}` — it must drive both engines"
            ));
        }
    }
    problems
}

fn is_exempt_path(rel: &str) -> bool {
    SANCTIONED_FILES.contains(&rel)
        || rel.contains("/tests/")
        || rel.ends_with("/tests.rs")
        || rel.contains("/examples/")
        || rel.contains("/benches/")
}

/// Every offending `(line_number, trimmed_line, why)` in `text`.
fn scan(text: &str) -> Vec<(usize, &str, &'static str)> {
    let lines: Vec<&str> = text.lines().collect();
    let test_tail = test_module_start(&lines).unwrap_or(lines.len());
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(test_tail) {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        let code = code_only(trimmed);
        let Some(why) = rule_for(&lines, idx, test_tail, &code) else {
            continue;
        };
        if !is_waived(&lines, idx) {
            out.push((idx + 1, trimmed, why));
        }
    }
    out
}

/// The rule this line breaks, if any.
fn rule_for(lines: &[&str], idx: usize, limit: usize, code: &str) -> Option<&'static str> {
    // Rule 1 is single-line by construction: a match pattern's alternation
    // of two character literals is never wrapped.
    if TERMINATOR_NEEDLES.iter().any(|n| code.contains(n)) {
        return Some(TERMINATOR_WHY);
    }
    // Rule 2 needs a window: rustfmt splits a long `matches!` across lines,
    // and the `prev` scrutinee sits *before* the alternation.
    if !code.contains("Sep") {
        return None;
    }
    let window = joined(
        lines,
        idx.saturating_sub(LOOKBEHIND),
        (idx + LOOKAHEAD).min(limit),
    );
    WORD_START_NEEDLES
        .iter()
        .filter_map(|n| exact_alternation(&window, n))
        .any(|at| scrutinee_is_previous_token(&window[..at]))
        .then_some(WORD_START_WHY)
}

/// Where `needle` appears in `window` as a **complete** alternation — the
/// two-variant word rule, not the prefix of a wider trivia set
/// (`Sep | Eol | Eof | Comment`). Returns the match offset.
fn exact_alternation(window: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(pos) = window[from..].find(needle) {
        let at = from + pos;
        let before = window[..at].trim_end().chars().next_back();
        let after = window[at + needle.len()..].trim_start().chars().next();
        if before != Some('|') && after != Some('|') {
            return Some(at);
        }
        from = at + needle.len();
    }
    None
}

/// Whether the innermost `matches!(` / `match ` opened before the
/// alternation is applied to a *previous-token* binding — the tell of a
/// state machine, as against a filter over the token in hand.
fn scrutinee_is_previous_token(before: &str) -> bool {
    let head = before
        .rfind("matches!(")
        .map(|at| at + "matches!(".len())
        .or_else(|| before.rfind("match ").map(|at| at + "match ".len()));
    let Some(head) = head else {
        return false;
    };
    mentions_prev(&before[head..])
}

/// Whether `text` contains an identifier that starts a `prev` word
/// (`prev`, `prev_type`, `prev_kind`, `previous_token`).
fn mentions_prev(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(pos) = text[from..].find("prev") {
        let at = from + pos;
        if at == 0 || !is_ident_byte(bytes[at - 1]) {
            return true;
        }
        from = at + 4;
    }
    false
}

const fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Lines `[from, to)` as one whitespace-collapsed code string, with
/// `tcl_lexer::` path prefixes normalised away.
fn joined(lines: &[&str], from: usize, to: usize) -> String {
    let mut out = String::new();
    for line in &lines[from..to.min(lines.len())] {
        out.push_str(&code_only(line.trim()));
        out.push(' ');
    }
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("tcl_lexer::", "")
}

/// The line of the `#[cfg(test)]` that annotates a **module**, after which
/// nothing is production code. (Verbatim from `dialect_drift`: a
/// `#[cfg(test)]` on a single `use` near the top must not end the scan.)
fn test_module_start(lines: &[&str]) -> Option<usize> {
    lines.iter().enumerate().find_map(|(i, l)| {
        if !l.trim_start().starts_with("#[cfg(test)]") {
            return None;
        }
        let next = lines[i + 1..]
            .iter()
            .map(|l| l.trim_start())
            .find(|l| !l.is_empty() && !l.starts_with("#["))?;
        let item = next.strip_prefix("pub ").unwrap_or(next);
        let item = item.strip_prefix("(crate) ").unwrap_or(item);
        (item.starts_with("mod ")).then_some(i)
    })
}

/// The code part of a line: everything before a `//` that is not inside a
/// string literal, with the *contents* of string literals elided — a needle
/// quoted in a message or a doc string is a mention, not a call.
fn code_only(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_str = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
                out.push('"');
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' if bytes.get(i + 1) == Some(&b'"') && bytes.get(i + 2) == Some(&b'\'') => {
                out.push_str("'\"'");
                i += 3;
                continue;
            }
            b'"' => {
                in_str = true;
                out.push('"');
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => break,
            _ => out.push(b as char),
        }
        i += 1;
    }
    out
}

/// A waiver is honoured on the flagged line itself, or anywhere in the run
/// of `//` comment lines directly above it. It does not leak past
/// intervening code.
fn is_waived(lines: &[&str], idx: usize) -> bool {
    if lines[idx].contains(WAIVER) {
        return true;
    }
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let t = lines[i].trim_start();
        if !t.starts_with("//") {
            return false;
        }
        if t.contains(WAIVER) {
            return true;
        }
    }
    false
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hand_rolled_terminator_scan_is_flagged() {
        let src = "fn f(b: &[u8]) {\n    while i < n {\n        match b[i] {\n            \
                   b'\\n' | b';' => out.push(i),\n            _ => i += 1,\n        }\n    }\n}\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].0, 4);
        assert!(hits[0].2.contains("command-terminator"));
    }

    #[test]
    fn both_orders_and_both_literal_kinds_are_flagged() {
        for src in [
            "match c { b';' | b'\\n' => {} }\n",
            "match c { '\\n' | ';' => {} }\n",
            "match c { ';' | '\\n' => {} }\n",
            "if matches!(bytes[p], b' ' | b'\\t' | b'\\n' | b';') { p += 1; }\n",
        ] {
            assert_eq!(scan(src).len(), 1, "{src:?}");
        }
    }

    #[test]
    fn a_terminator_that_is_not_a_pair_is_not_flagged() {
        // A newline alone, or a semicolon alone, is not the terminator set.
        assert!(scan("match c { b'\\n' => {} }\n").is_empty());
        assert!(scan("match c { b';' => {} }\n").is_empty());
        // A `\n` and a `;` in different patterns is not an alternation.
        assert!(scan("match c { b'\\n' => a(), b'x' | b';' => b() }\n").is_empty());
    }

    #[test]
    fn a_private_word_start_state_machine_is_flagged() {
        let src = "fn f(toks: &[Token]) {\n    let mut prev_type = TokenType::Eol;\n    \
                   for t in toks {\n        let starts = matches!(prev_type, TokenType::Sep | \
                   TokenType::Eol);\n    }\n}\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].0, 4);
        assert!(hits[0].2.contains("previous-token"));
    }

    #[test]
    fn the_wrapped_and_path_qualified_form_is_still_flagged() {
        let src = "fn f() {\n    let start = matches!(\n        prev_kind,\n        \
                   tcl_lexer::TokenType::Sep\n            | tcl_lexer::TokenType::Eol,\n    );\n}\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].0, 4, "flagged where the alternation starts");
    }

    #[test]
    fn trivia_skipping_is_not_a_word_start_machine() {
        // No `prev` binding: a filter, not a state machine.
        assert!(
            scan(
                "let words = toks.iter().filter(|t| !matches!(t.kind, TokenType::Sep | \
                  TokenType::Eol));\n"
            )
            .is_empty()
        );
        // A wider trivia set is not the two-variant word rule either.
        assert!(
            scan(
                "fn f() {\n let prev = x;\n if matches!(t.kind, TokenType::Sep | \
                  TokenType::Eol | TokenType::Eof) {}\n}\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_waiver_is_honoured_on_the_line_and_in_the_block_above() {
        assert!(
            scan(
                "match c { b'\\n' | b';' => {} } // segmentation-drift-ok: BIG-IP config, not Tcl\n"
            )
            .is_empty()
        );
        let src = "// segmentation-drift-ok: trims a terminator inside an already\n\
                   // segmented command span; the boundary came from the segmenter\n\
                   let a = matches!(b[i], b'\\n' | b';');\n\
                   let z = 1;\n\
                   let c = matches!(b[i], b'\\n' | b';');\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].0, 5,
            "the waiver does not leak past intervening code"
        );
    }

    #[test]
    fn a_mention_in_a_comment_or_string_is_not_a_hit() {
        assert!(scan("// the old shape was b'\\n' | b';'\n").is_empty());
        assert!(scan("let m = \"b'\\\\n' | b';'\";\n").is_empty());
    }

    #[test]
    fn the_test_tail_is_exempt() {
        let src = "fn f() {}\n#[cfg(test)]\nmod tests {\n    fn g(c: u8) { \
                   let _ = matches!(c, b'\\n' | b';'); }\n}\n";
        assert!(scan(src).is_empty());
    }

    #[test]
    fn owner_and_test_paths_are_exempt() {
        assert!(is_exempt_path("rust/tcl-lexer/src/script.rs"));
        assert!(is_exempt_path("rust/tcl-lexer/src/structural_index.rs"));
        assert!(is_exempt_path("rust/tcl-compiler/src/segmenter.rs"));
        assert!(is_exempt_path("rust/tcl-lexer/tests/lexer_depth.rs"));
        assert!(!is_exempt_path("rust/tcl-lsp-core/src/expr_context.rs"));
        assert!(!is_exempt_path("rust/tcl-compiler/src/lowering/mod.rs"));
    }

    #[test]
    fn the_cross_check_is_required_to_exist_and_name_both_engines() {
        let root = crate::util::repo_root();
        assert!(
            cross_check_problems(&root).is_empty(),
            "{:?}",
            cross_check_problems(&root)
        );
        assert_eq!(cross_check_problems(Path::new("/nonexistent")).len(), 1);
    }
}
