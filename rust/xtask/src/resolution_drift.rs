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

//! `resolution-drift` — the name-resolution drift gate.
//!
//! The M1 root cause of the workspace's wrong-symbol bugs was consumers
//! scanning `all_procs` / `all_classes` with a namespace-blind simple-name
//! compare (`.name == word`) instead of the shared resolution contract
//! (`tcl_syntax::naming` candidates, `definition.rs`'s sanctioned helpers) —
//! 17 sites had drifted before the contract landed.  The "add a vector"
//! discipline only protects consumers already inside the contract, so this
//! lint (grep-shaped, per the plan) flags any **new** `.name ==` compare in
//! the lexical neighbourhood of an `all_procs` / `all_classes` scan.
//!
//! Escapes:
//! - the contract's own implementation files are exempt (see
//!   [`SANCTIONED_FILES`]);
//! - a deliberate, reviewed scan carries a `// drift-ok: <reason>` comment on
//!   the flagged line or one of the two lines above it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Files allowed to implement simple-name scans: the resolution contract
/// itself and the documented deterministic fallback it exposes.
const SANCTIONED_FILES: &[&str] = &[
    "rust/tcl-syntax/src/naming.rs",
    "rust/tcl-lsp-core/src/definition.rs",
    // The lint's own docs and fixtures spell the pattern out.
    "rust/xtask/src/resolution_drift.rs",
];

/// How many bytes after an `all_procs` / `all_classes` mention a `.name ==`
/// compare is considered part of the same scan expression.  Wide enough for a
/// multi-line `.values().find(|p| p.name == …)` chain, narrow enough not to
/// leap into unrelated statements.
const WINDOW: usize = 240;

/// Scan the workspace's Rust sources; exit non-zero listing any offending
/// site.  `check` is accepted for CLI symmetry with the other gates — the
/// lint never rewrites anything, so both modes verify.
pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    collect_rs_files(&root.join("runtime/rust/src"), &mut files);

    let mut report = String::new();
    let mut hits = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // Test code pins behaviour rather than resolving names — skip
        // integration-test trees and test-only modules.
        if SANCTIONED_FILES.contains(&rel.as_str())
            || rel.contains("/tests/")
            || rel.ends_with("/tests.rs")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, window_text) in scan_windows(&text) {
            if window_is_waived(&text, line_no) {
                continue;
            }
            hits += 1;
            let snippet = window_text
                .lines()
                .next()
                .unwrap_or(window_text)
                .trim()
                .chars()
                .take(96)
                .collect::<String>();
            let _ = writeln!(report, "  {rel}:{line_no}: {snippet}");
        }
    }

    if hits == 0 {
        println!("resolution-drift: OK (no namespace-blind `.name ==` scans over all_procs/all_classes)");
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "resolution-drift: {hits} namespace-blind simple-name scan(s) over \
         all_procs/all_classes — resolve through `tcl_syntax::naming` \
         candidates or `definition.rs`'s sanctioned helpers, or mark a \
         reviewed exception with `// drift-ok: <reason>`:\n{report}"
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

/// Yield `(1-based line, window text)` for every `all_procs`/`all_classes`
/// mention whose following [`WINDOW`] bytes contain a `.name ==` / `== x.name`
/// compare (`.qualified_name ==` never matches — the dot must immediately
/// precede `name`).
fn scan_windows(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    for needle in ["all_procs", "all_classes"] {
        let mut from = 0;
        while let Some(off) = text[from..].find(needle) {
            let start = from + off;
            from = start + needle.len();
            let end = (start + WINDOW).min(text.len());
            let end = (start..=end)
                .rev()
                .find(|&i| text.is_char_boundary(i))
                .unwrap_or(start);
            let window = &text[start..end];
            if has_simple_name_compare(window) {
                let line_no = text[..start].bytes().filter(|&b| b == b'\n').count() + 1;
                out.push((line_no, window));
            }
        }
    }
    out.sort_unstable_by_key(|&(l, _)| l);
    out.dedup_by_key(|&mut (l, _)| l);
    out
}

/// Whether `window` contains a `.name ==` or `== <expr>.name` compare.
fn has_simple_name_compare(window: &str) -> bool {
    let bytes = window.as_bytes();
    let mut from = 0;
    while let Some(off) = window[from..].find(".name") {
        let start = from + off;
        from = start + ".name".len();
        // The token must END at `.name` (`.name_span` / `.names` differ).
        let after = &bytes[from..];
        let word_continues = after
            .first()
            .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
        if word_continues {
            continue;
        }
        let rest = window[from..].trim_start();
        // Reversed operand order (`word == c.name`): strip the receiver
        // expression's trailing identifier path back to the operator.
        let before = window[..start]
            .trim_end_matches(|ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
            .trim_end();
        if rest.starts_with("==") || before.ends_with("==") {
            return true;
        }
    }
    false
}

/// Whether the flagged line (or one of the four lines above it — room for a
/// multi-line justification comment) carries a `drift-ok:` waiver.
fn window_is_waived(text: &str, line_no: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let idx = line_no.saturating_sub(1);
    (idx.saturating_sub(4)..=idx)
        .filter_map(|i| lines.get(i))
        .any(|l| l.contains("drift-ok:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_the_blind_scan_shape() {
        let src = "let p = analysis.all_procs.values().find(|p| p.name == head);\n";
        assert_eq!(scan_windows(src).len(), 1);
    }

    #[test]
    fn multiline_chain_is_still_inside_the_window() {
        let src = "let q = analysis\n    .all_procs\n    .values()\n    .find(|p| p.name == word);\n";
        assert_eq!(scan_windows(src).len(), 1);
    }

    #[test]
    fn qualified_and_span_compares_do_not_match() {
        for ok in [
            "all_procs.values().find(|p| p.qualified_name == head)",
            "all_procs.values().find(|p| p.name_span == span)",
            "all_classes.get_key_value(&cand)",
            "all_procs.iter().filter(|(_, p)| p.name.starts_with(word))",
        ] {
            assert!(scan_windows(ok).is_empty(), "{ok}");
        }
    }

    #[test]
    fn reversed_operand_order_matches() {
        let src = "all_classes.values().any(|c| word == c.name)";
        assert_eq!(scan_windows(src).len(), 1);
    }

    #[test]
    fn waiver_comment_suppresses() {
        let src = "// drift-ok: deterministic fallback, documented\nlet p = all_procs.values().find(|p| p.name == w);\n";
        let hits = scan_windows(src);
        assert_eq!(hits.len(), 1);
        assert!(window_is_waived(src, hits[0].0));
    }
}
