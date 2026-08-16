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
//! The workspace therefore has exactly one numeral **value parser**,
//! [`tcl_syntax::number`], parameterised by `NumberSyntax`, and one expression
//! **lexeme-boundary scanner**, [`tcl_dialect::scan_expr_number`]. Everything
//! else asks one of those owners. This lint exists because the value-parser
//! rule was broken six separate times, each independently and each with the
//! same shape — strip a two-character radix prefix by hand, then call
//! `from_str_radix`:
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
//! It also verifies the expression lexer and expression-number classifier both
//! route through the lower `tcl-dialect` boundary owner, rejects another source
//! declaring the owner's entry-point names, and requires the owner/consumer
//! regression corpus for `1_eq`, radix-operator junctions, and NaN payloads.
//! This is structural wiring evidence, not a proof that arbitrary Rust text
//! cannot reproduce the semantics; review and the mutation-sensitive tests
//! remain the semantic backstop. The same check pins `scan_nan_payload`, because
//! its whitespace and thirteen-digit ceiling must agree between lexing and
//! `tcl_syntax::number`.
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

/// Files allowed to recognise a Tcl radix prefix: the value parser, the lower
/// expression-boundary scanner, and this lint's own docs and fixtures.
const SANCTIONED_FILES: &[&str] = &[
    "rust/tcl-dialect/src/expr_number.rs",
    "rust/tcl-syntax/src/number.rs",
    "rust/xtask/src/number_drift.rs",
];

/// The lower owner of `ParseLexeme`'s expression-number boundary half.
const EXPR_BOUNDARY_OWNER: &str = "rust/tcl-dialect/src/expr_number.rs";

/// Higher layers that must call the boundary owner rather than scan again.
const EXPR_BOUNDARY_CONSUMERS: &[&str] = &[
    "rust/tcl-lexer/src/expr_lexer.rs",
    "rust/tcl-syntax/src/number.rs",
];

/// The NaN payload sub-owner is shared by the lower scanner and value parser.
const NAN_PAYLOAD_CONSUMER: &str = "rust/tcl-syntax/src/number.rs";

/// Named regression rows that make the lower-owner wiring evidence semantic.
/// The gate deliberately checks their presence rather than pretending a text
/// scan proves semantic uniqueness; the Rust tests execute the actual rows.
const EXPR_BOUNDARY_CORPUS: &[(&str, &str)] = &[
    (
        EXPR_BOUNDARY_OWNER,
        "number_bareword_junctions_match_tcl_parselexeme",
    ),
    (
        EXPR_BOUNDARY_OWNER,
        "tcl84_getlexeme_keeps_integer_prefixes_separate",
    ),
    (
        EXPR_BOUNDARY_OWNER,
        "explicit_radix_numerals_split_only_before_available_word_operators",
    ),
    (
        EXPR_BOUNDARY_OWNER,
        "special_floats_follow_numeric_junction_rules",
    ),
    (
        "rust/tcl-lexer/src/expr_lexer.rs",
        "explicit_radix_numbers_stop_before_word_operators",
    ),
    (
        "rust/tcl-lexer/src/expr_lexer.rs",
        "completed_nan_payload_is_a_number_before_following_bareword",
    ),
    (
        NAN_PAYLOAD_CONSUMER,
        "expression_number_validation_shares_the_boundary_owner",
    ),
];

/// The radix-prefix spellings a Tcl numeral can carry, in both cases.
const RADIX_PREFIXES: &[&str] = &["0x", "0X", "0o", "0O", "0b", "0B", "0d", "0D"];

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

    for problem in expr_boundary_owner_problems(&root) {
        hits += 1;
        let _ = writeln!(report, "  {problem}");
    }

    if hits == 0 {
        println!(
            "number-drift: OK (no hand-rolled Tcl radix-prefix recognition \
             outside tcl_syntax::number; expr lexeme boundaries use \
             tcl_dialect::scan_expr_number)"
        );
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "number-drift: {hits} site(s) recognising a Tcl radix prefix by hand. \
         Tcl's numeral grammar is release-dependent (`0b`/`0o` are 8.5+, `0d` \
         and `_` are 9.0+, leading-zero octal ends at 9.0), so a hand-rolled \
         parser is silently wrong for some release. Route value parsing through \
         `tcl_syntax::number` with the target's `NumberSyntax`, and expression \
         lexeme boundaries through `tcl_dialect::scan_expr_number` — or, if the \
         text is not Tcl script (a hex colour, a packet field, a hex-encoded \
         byte string), mark it `// number-drift-ok: <reason>`:\n{report}"
    );
    ExitCode::FAILURE
}

/// Check the dependency-safe split between value parsing and expression lexing.
fn expr_boundary_owner_problems(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let owner_text = read_rust_source(root, EXPR_BOUNDARY_OWNER);
    if !owner_text.as_ref().is_some_and(|text| {
        text.lines()
            .any(|line| line.trim_start().starts_with("pub fn scan_expr_number("))
    }) {
        problems.push(format!(
            "{EXPR_BOUNDARY_OWNER}: missing public expression-number boundary owner"
        ));
    }
    for consumer in EXPR_BOUNDARY_CONSUMERS {
        if !read_rust_source(root, consumer)
            .is_some_and(|text| text.contains("tcl_dialect::scan_expr_number("))
        {
            problems.push(format!(
                "{consumer}: expression-number consumer does not call \
                 tcl_dialect::scan_expr_number"
            ));
        }
    }
    if !owner_text.as_ref().is_some_and(|text| {
        text.lines()
            .any(|line| line.trim_start().starts_with("pub fn scan_nan_payload("))
    }) {
        problems.push(format!(
            "{EXPR_BOUNDARY_OWNER}: missing public NaN payload boundary owner"
        ));
    }
    if !read_rust_source(root, NAN_PAYLOAD_CONSUMER)
        .is_some_and(|text| text.contains("tcl_dialect::scan_nan_payload("))
    {
        problems.push(format!(
            "{NAN_PAYLOAD_CONSUMER}: NaN value parser does not call \
             tcl_dialect::scan_nan_payload"
        ));
    }
    if !read_rust_source(root, NAN_PAYLOAD_CONSUMER).is_some_and(|text| {
        production_function_contains_call(&text, "is_expr_number", "tcl_dialect::scan_expr_number(")
    }) {
        problems.push(format!(
            "{NAN_PAYLOAD_CONSUMER}: expression-number value classifier does not \
             call tcl_dialect::scan_expr_number in its production body"
        ));
    }
    for &(path, test_name) in EXPR_BOUNDARY_CORPUS {
        if !read_rust_source(root, path)
            .is_some_and(|text| text.contains(&format!("fn {test_name}(")))
        {
            problems.push(format!(
                "{path}: missing expression-number regression row `{test_name}`"
            ));
        }
    }

    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    collect_rs_files(&root.join("runtime/rust/src"), &mut files);
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel == EXPR_BOUNDARY_OWNER || rel.contains("/tests/") || rel.ends_with("/tests.rs") {
            continue;
        }
        if read_rust_source(root, &rel).is_some_and(|text| {
            declares_expr_boundary_scanner(&text) || declares_nan_payload_scanner(&text)
        }) {
            problems.push(format!(
                "{rel}: duplicate expression-number or NaN-payload boundary scanner; \
                 use tcl_dialect's shared owner"
            ));
        }
    }
    problems
}

fn declares_expr_boundary_scanner(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("fn scan_expr_number(") || line.starts_with("pub fn scan_expr_number(")
    })
}

fn declares_nan_payload_scanner(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("fn scan_nan_payload(") || line.starts_with("pub fn scan_nan_payload(")
    })
}

/// Return whether a named production function's body contains `call`.
///
/// The owner-wiring gate must inspect the production call site, not merely a
/// test fixture elsewhere in the same source file. This intentionally small
/// brace matcher is sufficient for Rust function bodies and avoids treating a
/// test-only mention as evidence that the production classifier still routes
/// through the shared boundary owner.
fn production_function_contains_call(text: &str, function: &str, call: &str) -> bool {
    let needle = format!("fn {function}(");
    let Some(signature) = text.find(&needle) else {
        return false;
    };
    let Some(open_rel) = text[signature..].find('{') else {
        return false;
    };
    let open = signature + open_rel;
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return text[open..open + offset].contains(call);
                }
            }
            _ => {}
        }
    }
    false
}

fn read_rust_source(root: &Path, relative: &str) -> Option<String> {
    std::fs::read_to_string(root.join(relative)).ok()
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

    #[test]
    fn expression_boundary_owner_is_unique_and_consumed() {
        assert!(expr_boundary_owner_problems(&crate::util::repo_root()).is_empty());
    }

    #[test]
    fn scanner_declaration_check_ignores_a_string_mention() {
        assert!(!declares_expr_boundary_scanner(
            "let name = \"fn scan_expr_number(\";"
        ));
        assert!(declares_expr_boundary_scanner(
            "pub fn scan_expr_number(source: &[u8]) {}"
        ));
    }

    #[test]
    fn nan_payload_declaration_check_ignores_a_string_mention() {
        assert!(!declares_nan_payload_scanner(
            "let name = \"fn scan_nan_payload(\";"
        ));
        assert!(declares_nan_payload_scanner(
            "pub fn scan_nan_payload(source: &[u8]) {}"
        ));
    }

    #[test]
    fn production_call_check_ignores_test_only_mentions() {
        let test_only = "fn is_expr_number(text: &str) -> bool {\n\
                         is_whole_number(text)\n\
                     }\n\
                     #[test]\n\
                     fn wiring() { tcl_dialect::scan_expr_number(b\"1\", 0, s, None); }\n";
        assert!(!production_function_contains_call(
            test_only,
            "is_expr_number",
            "tcl_dialect::scan_expr_number("
        ));

        let production = "fn is_expr_number(text: &str) -> bool {\n\
                          tcl_dialect::scan_expr_number(text.as_bytes(), 0, s, None)\
                              .is_some()\n\
                      }\n";
        assert!(production_function_contains_call(
            production,
            "is_expr_number",
            "tcl_dialect::scan_expr_number("
        ));
    }
}
