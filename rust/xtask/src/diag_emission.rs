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

//! `diag-emission-check` — the "every non-reserved code has a producer"
//! completeness gate (issue #1317).
//!
//! `docs/design/rust/cleanup-status.md` used to explain that this guard was
//! deliberately *not* added, because it would false-positive on codes that
//! are genuinely specified-but-not-yet-wired (`W130`-`W134`). Now that
//! `DocRow::Diagnostic`'s `reserved` field lets a code declare that state
//! honestly, the guard can exist without that false-positive: it skips
//! every `internal` code (always active by construction) and every
//! `reserved` code (declared as having no producer yet), and for
//! everything else, greps [`SEARCH_ROOTS`] for at least one real
//! construction site.
//!
//! This is exactly the class of bug issue #1317 found twice: `W122` had no
//! producer left at all (its check was superseded by `W124` and never
//! removed), and `W308`'s registry entry had drifted to describe a check
//! (`subst` without `-nocommands`) no emitter ever implemented. A code that
//! silently loses its only producer — or never had one to begin with — ships
//! to every editor as a working, toggleable setting that quietly does
//! nothing. This gate catches that the moment it happens instead of waiting
//! for the next false-positive audit to notice.
//!
//! **Heuristic, not proof.** This is a source grep (matching the project's
//! existing `resolution-drift` gate's own style), not points-to analysis: it
//! looks for either the `DiagCode::<Variant>` construction pattern or the
//! bare quoted wire spelling (`"O111"`, for the handful of codes a consumer
//! synthesises by hand outside the `DiagCode` enum) anywhere in the
//! non-test sources under [`SEARCH_ROOTS`]. A code mentioned only in a doc
//! comment or an unrelated string would false-negative (report "has a
//! producer" when it doesn't) — a real gap this gate cannot close — but it
//! reliably catches the "zero references left anywhere in real code" case,
//! which is what both W122 and (before the fact) W308 actually were.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tcl_core_types::DiagCode;

/// Directories under the repo root that hold real emission sites — scanned
/// for at least one construction per non-internal, non-reserved code. Most
/// codes are constructed in `tcl-compiler` (the analyser / compiler-checks /
/// optimiser); a handful (`W111`/`W112`/`W115`/`W118`, the pure-text style
/// checks) are built directly in `tcl-lsp-core::source_style`, and a couple
/// (e.g. `O111`, paired onto `W100` at publish time) are synthesised by hand
/// in the LSP server rather than going through `DiagCode` construction at
/// all.
const SEARCH_ROOTS: &[&str] = &[
    "rust/tcl-compiler/src",
    "rust/tcl-lsp-core/src",
    "rust/tcl-lsp-server/src",
];

/// Recursively collect `.rs` files under `dir`, skipping `target/`, test
/// trees (`/tests/`, `tests.rs`), and the FP regression-test module (`/fp/`
/// — pure `#[test]` fixtures, not emission sites; see `analyser/diagnostics/fp/`).
fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == "tests" || name == "fp" {
                continue;
            }
            collect_source_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs")
            && path.file_name().is_some_and(|n| n != "tests.rs")
        {
            out.push(path);
        }
    }
}

/// The `DiagCode` enum-variant spelling for a wire code (`"IRULE1001"` ->
/// `"Irule1001"`, `"W124"` -> `"W124"`): the leading alphabetic run
/// title-cased, the trailing digit run unchanged — the convention every
/// variant in `diag_code.rs` follows.
fn variant_ident(code: &str) -> String {
    let digits_at = code
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(code.len());
    let (alpha, digits) = code.split_at(digits_at);
    let mut out = String::with_capacity(code.len());
    let mut chars = alpha.chars();
    if let Some(first) = chars.next() {
        out.extend(first.to_uppercase());
    }
    out.extend(chars.flat_map(char::to_lowercase));
    out.push_str(digits);
    out
}

/// Whether `text` contains `needle` at a Rust-identifier boundary on both
/// sides (so matching `"W124"` inside `"W1244"`, or `DiagCode::W124` inside
/// `DiagCode::W1244`, doesn't count).
fn contains_at_boundary(text: &str, needle: &str) -> bool {
    let bytes = text.as_bytes();
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut start = 0;
    while let Some(off) = text[start..].find(needle) {
        let idx = start + off;
        let end = idx + needle.len();
        let left_ok = idx == 0 || !is_ident(bytes[idx - 1]);
        let right_ok = bytes.get(end).is_none_or(|&b| !is_ident(b));
        if left_ok && right_ok {
            return true;
        }
        start = end.max(idx + 1);
    }
    false
}

/// Whether `text` constructs `code` — either through `DiagCode::<Variant>`
/// or, for the handful of codes a consumer synthesises by hand outside the
/// `DiagCode` enum (see [`SEARCH_ROOTS`]), the bare quoted wire spelling
/// (`"O111"`). The quoted form is a looser heuristic (a stray comment or
/// string could false-positive it into "has a producer") — an acceptable
/// trade-off, since a false positive here only weakens the guard, it never
/// flags a real producer as missing.
fn contains_construction(text: &str, code: &str) -> bool {
    let variant_needle = format!("DiagCode::{}", variant_ident(code));
    if contains_at_boundary(text, &variant_needle) {
        return true;
    }
    let string_needle = format!("\"{code}\"");
    text.contains(&string_needle)
}

/// Run the gate: exit non-zero listing any non-internal, non-reserved code
/// with no construction site found.
pub fn run() -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    for search_root in SEARCH_ROOTS {
        collect_source_files(&root.join(search_root), &mut files);
    }
    let sources: Vec<String> = files
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();

    let mut missing: Vec<&'static str> = Vec::new();
    for code in DiagCode::ALL {
        if code.is_internal() || code.is_reserved() {
            continue;
        }
        let found = sources
            .iter()
            .any(|text| contains_construction(text, code.as_str()));
        if !found {
            missing.push(code.as_str());
        }
    }

    if missing.is_empty() {
        println!(
            "diag-emission-check: OK ({} non-internal, non-reserved code(s) each have at \
             least one construction site under {})",
            DiagCode::ALL
                .iter()
                .filter(|c| !c.is_internal() && !c.is_reserved())
                .count(),
            SEARCH_ROOTS.join(", "),
        );
        return ExitCode::SUCCESS;
    }

    let mut report = String::new();
    for code in &missing {
        let _ = writeln!(report, "  {code}");
    }
    eprintln!(
        "diag-emission-check: {} code(s) declared in DiagCode::ALL (non-internal, \
         non-reserved) have no construction site under {} — either the code lost \
         its only producer (fix the emitter, or retire the code) or it should be \
         marked `reserved` if it is a genuinely not-yet-implemented, specified \
         diagnostic:\n{report}",
        missing.len(),
        SEARCH_ROOTS.join(", "),
    );
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_ident_single_letter_prefix_is_unchanged() {
        assert_eq!(variant_ident("W124"), "W124");
        assert_eq!(variant_ident("O111"), "O111");
        assert_eq!(variant_ident("E002"), "E002");
    }

    #[test]
    fn variant_ident_multi_letter_prefix_is_title_cased() {
        assert_eq!(variant_ident("IRULE1001"), "Irule1001");
        assert_eq!(variant_ident("TK1001"), "Tk1001");
    }

    #[test]
    fn construction_matches_diag_code_variant() {
        let src = "diagnostics.push(Diagnostic { code: DiagCode::W124, .. });";
        assert!(contains_construction(src, "W124"));
    }

    #[test]
    fn construction_matches_irule_variant_casing() {
        let src = "code: DiagCode::Irule1001,";
        assert!(contains_construction(src, "IRULE1001"));
    }

    #[test]
    fn construction_matches_bare_quoted_wire_string() {
        // O111 (paired onto W100 by hand in the LSP server) has no
        // `DiagCode::O111` construction anywhere — only the quoted spelling.
        let src = r#"code: Some(NumberOrString::String("O111".to_string())),"#;
        assert!(contains_construction(src, "O111"));
    }

    #[test]
    fn construction_does_not_false_positive_on_a_longer_code() {
        // FP guard: DiagCode::W1244 (hypothetical) must not satisfy a search
        // for W124, and neither must the quoted string "W1244".
        let src = "DiagCode::W1244; let s = \"W1244\";";
        assert!(!contains_construction(src, "W124"));
    }

    #[test]
    fn construction_absent_when_code_never_appears() {
        let src = "code: DiagCode::W120,";
        assert!(!contains_construction(src, "W124"));
    }

    #[test]
    fn every_non_internal_non_reserved_code_has_a_producer() {
        // The gate itself, as a `cargo test`-reachable regression (not just
        // a `cargo xtask` subcommand) — so a future PR that silently drops
        // a code's last producer (the W122 / W308 class of bug, issue
        // #1317) fails `cargo test -p xtask`, not just the drift gate.
        assert_eq!(run(), ExitCode::SUCCESS);
    }
}
