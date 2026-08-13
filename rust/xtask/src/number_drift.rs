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

//! `number-drift` — the numeric-grammar drift gate.
//!
//! Tcl's numeral grammar changes between releases: the `0b`/`0o` prefixes
//! arrive in 8.5, `0d` and `_` digit separators in 9.0, and octal-by-leading-
//! zero is retired *in* 9.0 (`0755` is 493 up to 8.6 and 755 from 9.0). A
//! parser that hard-codes one release's spelling set is wrong for every other
//! release, and — worse — wrong *silently*, because most numerals read the same
//! either way.
//!
//! The workspace therefore has exactly one numeral parser,
//! [`tcl_syntax::number`], parameterised by `NumberSyntax`. Everything else
//! asks it. This lint exists because that rule was broken six separate times
//! before it was enforced, each independently and each with the same shape —
//! strip a two-character radix prefix by hand, then call `from_str_radix`:
//!
//! - `bpf-tcl-ir`'s eBPF lowering — rejected valid 9.x numerals
//! - `tcl-compiler`'s interval analysis — read `0755` as 755 for an 8.6 target
//! - `tcl-compiler`'s type inference — could not see `0o`/`0d`/`_` at all
//! - `tcl-lsp-core`'s semantic tokens — did not highlight `0o17` as a number
//! - `tcl-lsp-core`'s inline-proc refactor — accepted `0xZZZ` as a numeral
//! - `tcl-registry`'s frame-level words — four spellings wrong across releases
//!
//! # What it flags
//!
//! A Tcl radix-prefix literal (`"0x"`, `"0o"`, `"0b"`, `"0d"`, any case) used
//! to **recognise** a numeral: as the argument of `strip_prefix`/`starts_with`,
//! as a `match` arm pattern, or in an equality against a string slice.
//!
//! It deliberately does *not* flag code that **produces** such a prefix —
//! `format`'s `%#x` inserting `"0x"` ahead of its digits is not numeral
//! recognition — nor a `from_str_radix` whose radix comes from somewhere real
//! (a `scan` conversion character, a `format` spec), nor prefix literals
//! appearing as test data. That keeps the signal about the one drift class.
//!
//! # Known blind spot
//!
//! A scanner that dispatches on the prefix *character* rather than a two-byte
//! string literal — `match c { 'x' => radix = 16, 'o' => radix = 8, … }` — is
//! not flagged, and `tcl-cmd-core/src/string_is.rs` is a live example. The
//! pattern cannot be gated without noise: mapping a conversion character to a
//! radix is exactly what `format`'s `%x`/`%o` and `scan`'s converters
//! legitimately do, so the same shape is correct in one place and drift in
//! another. Widening this lint to cover it would produce more waivers than
//! findings.
//!
//! Escapes:
//! - the facility's own implementation is exempt (see [`SANCTIONED_FILES`]);
//! - integration-test trees and test-only modules are skipped;
//! - a deliberate, reviewed site carries a `// number-drift-ok: <reason>`
//!   marker on the flagged line, or anywhere in the comment block directly
//!   above it. Parsing a radix prefix out of something that is *not* Tcl script
//!   text — a `tshark` field, a hex colour, a hex-encoded byte string — is the
//!   legitimate case, as is recognising a *spelling* without parsing it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Files allowed to recognise a Tcl radix prefix: the one numeral parser, and
/// this lint's own docs and fixtures, which spell the pattern out.
const SANCTIONED_FILES: &[&str] = &[
    "rust/tcl-syntax/src/number.rs",
    "rust/xtask/src/number_drift.rs",
];

/// The radix-prefix spellings a Tcl numeral can carry, in both cases.
const RADIX_PREFIXES: &[&str] = &[
    "0x", "0X", "0o", "0O", "0b", "0B", "0d", "0D",
];

/// Call shapes that mean "test whether this text starts with a radix prefix".
const RECOGNISERS: &[&str] = &["strip_prefix(", "starts_with(", "trim_start_matches("];

/// Scan the workspace's Rust sources; exit non-zero listing any offending site.
/// `check` is accepted for CLI symmetry with the other gates — the lint never
/// rewrites anything, so both modes verify.
pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    collect_rs_files(&root.join("runtime/rust/src"), &mut files);
    files.sort();

    let mut report = String::new();
    let mut hits = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if SANCTIONED_FILES.contains(&rel.as_str())
            || rel.contains("/tests/")
            || rel.ends_with("/tests.rs")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, snippet) in scan(&text) {
            if line_is_waived(&text, line_no) {
                continue;
            }
            hits += 1;
            let _ = writeln!(report, "  {rel}:{line_no}: {snippet}");
        }
    }

    if hits == 0 {
        println!(
            "number-drift: OK (no hand-rolled Tcl radix-prefix recognition \
             outside tcl_syntax::number)"
        );
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "number-drift: {hits} site(s) recognising a Tcl radix prefix by hand. \
         Tcl's numeral grammar is release-dependent (`0b`/`0o` are 8.5+, `0d` \
         and `_` are 9.0+, leading-zero octal ends at 9.0), so a hand-rolled \
         parser is silently wrong for some release. Route it through \
         `tcl_syntax::number` with the target's `NumberSyntax` — or, if the \
         text is not Tcl script (a hex colour, a packet field, a hex-encoded \
         byte string), mark it `// number-drift-ok: <reason>`:\n{report}"
    );
    ExitCode::FAILURE
}

/// Recursively collect `.rs` files under `dir` (skipping `target/`).
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

/// Yield `(1-based line, trimmed line text)` for every radix-prefix literal in
/// a recognition position.
fn scan(text: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for prefix in RADIX_PREFIXES {
        let quoted = format!("\"{prefix}\"");
        let mut from = 0;
        while let Some(off) = text[from..].find(&quoted) {
            let start = from + off;
            from = start + quoted.len();
            if !is_recognition_position(text, start, from) {
                continue;
            }
            let line_no = text[..start].bytes().filter(|&b| b == b'\n').count() + 1;
            let snippet = text[..start]
                .rfind('\n')
                .map_or(&text[..from], |nl| &text[nl + 1..from])
                .trim()
                .chars()
                .take(96)
                .collect::<String>();
            out.push((line_no, snippet));
        }
    }
    out.sort_unstable();
    out.dedup_by_key(|&mut (l, _)| l);
    out
}

/// Whether the literal spanning `start..end` is being used to *recognise* a
/// numeral rather than to produce or describe one.
fn is_recognition_position(text: &str, start: usize, end: usize) -> bool {
    let before = &text[..start];
    // `strip_prefix("0x")` / `starts_with("0x")`, possibly behind whitespace.
    let trimmed_before = before.trim_end();
    if RECOGNISERS.iter().any(|r| trimmed_before.ends_with(r)) {
        return true;
    }
    // `== "0x"` / `!= "0x"`.
    if trimmed_before.ends_with("==") || trimmed_before.ends_with("!=") {
        return true;
    }
    // A `match` arm: `"0x" => …`, or one alternative of `Some("0x" | "0X") =>`.
    // Walk forward over further `| "0Y"` alternatives and closing parens.
    let mut rest = text[end..].trim_start();
    loop {
        if rest.starts_with("=>") {
            return true;
        }
        if let Some(r) = rest.strip_prefix(')') {
            rest = r.trim_start();
            continue;
        }
        if let Some(r) = rest.strip_prefix('|') {
            let r = r.trim_start();
            // Only skip a *string-literal* alternative; `|` elsewhere is a
            // closure parameter list or a bitwise or.
            if let Some(r) = r.strip_prefix('"')
                && let Some(q) = r.find('"')
            {
                rest = r[q + 1..].trim_start();
                continue;
            }
            return false;
        }
        return false;
    }
}

/// Whether the flagged line carries a `number-drift-ok:` waiver — on the line
/// itself, or anywhere in the contiguous `//` comment block directly above it.
///
/// The whole attached block counts, rather than a fixed number of lines above:
/// a waiver here has to explain *why* the text being parsed is not Tcl script,
/// and that is usually several sentences. A fixed window silently stops
/// recognising the marker once the justification grows past it, which punishes
/// exactly the thorough explanations the waiver is supposed to require.
fn line_is_waived(text: &str, line_no: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let idx = line_no.saturating_sub(1);
    if lines.get(idx).is_some_and(|l| l.contains(WAIVER)) {
        return true;
    }
    // Walk up through the comment block immediately above the flagged line.
    lines[..idx.min(lines.len())]
        .iter()
        .rev()
        .take_while(|l| l.trim_start().starts_with("//"))
        .any(|l| l.contains(WAIVER))
}

/// The marker that waives a site.
const WAIVER: &str = "number-drift-ok:";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_the_strip_prefix_shape() {
        let src = "let r = t.strip_prefix(\"0x\").or_else(|| t.strip_prefix(\"0X\"));\n";
        assert_eq!(scan(src).len(), 1, "both hits are on one line");
    }

    #[test]
    fn flags_starts_with_and_equality_and_match_arms() {
        for bad in [
            "if low.starts_with(\"0o\") { }",
            "if prefix == \"0x\" { }",
            "match m.get(..2) { Some(\"0x\" | \"0X\") => 16, _ => 10 }",
            "match p { \"0b\" => 2, _ => 10 }",
        ] {
            assert_eq!(scan(bad).len(), 1, "{bad}");
        }
    }

    /// Emitting a prefix is not recognising one — `format`'s `%#x` prepends
    /// `"0x"` to its digits and must not be flagged.
    #[test]
    fn producing_a_prefix_is_not_flagged() {
        for ok in [
            "digits.insert_str(0, \"0d\");",
            "let sigil = if n != 0 { \"0x\" } else { \"\" };",
            "Radix::HexLower if n != 0 => \"0x\",",
            "b'x' | b'X' => b\"0x\",",
        ] {
            assert!(scan(ok).is_empty(), "{ok}");
        }
    }

    /// Prefix spellings as test data are not recognition sites.
    #[test]
    fn prefix_literals_as_test_data_are_not_flagged() {
        for ok in [
            "for spec in [\"1.0\", \"1+\", \"0x\", \"0xg\", \"2e0\"] { }",
            "for bad in [\"0x\", \"0b2\"] { }",
            "(\"0x\", None),",
        ] {
            assert!(scan(ok).is_empty(), "{ok}");
        }
    }

    #[test]
    fn waiver_comment_suppresses() {
        let src = "// number-drift-ok: tshark field, not Tcl script text\n\
                   let v = s.strip_prefix(\"0x\");\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1);
        assert!(line_is_waived(src, hits[0].0));
    }

    /// The marker counts anywhere in the attached comment block, however long
    /// the justification runs — a fixed lookback would stop seeing it.
    #[test]
    fn waiver_is_found_through_a_long_justification() {
        let src = "fn f(s: &str) -> bool {\n\
                   \x20   // number-drift-ok: first line of the reason\n\
                   \x20   // second line\n\
                   \x20   // third line\n\
                   \x20   // fourth line\n\
                   \x20   // fifth line\n\
                   \x20   s.starts_with(\"0o\")\n\
                   }\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1, "the site is still detected");
        assert!(line_is_waived(src, hits[0].0), "waiver must be honoured");
    }

    /// A waiver on some *unrelated* earlier statement must not carry over: the
    /// walk stops at the first non-comment line above the site.
    #[test]
    fn waiver_does_not_leak_past_intervening_code() {
        let src = "// number-drift-ok: applies to the line below only\n\
                   let a = other.strip_prefix(\"0x\");\n\
                   let b = thing.strip_prefix(\"0o\");\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 2);
        assert!(line_is_waived(src, hits[0].0), "first site is waived");
        assert!(!line_is_waived(src, hits[1].0), "second site is not");
    }

    /// A `|` that is not a string-literal alternative must not be mistaken for
    /// a match-arm continuation.
    #[test]
    fn closure_params_after_a_prefix_literal_do_not_match() {
        let src = "let n = pick(\"0x\") | mask(y);\n";
        assert!(scan(src).is_empty());
    }
}
