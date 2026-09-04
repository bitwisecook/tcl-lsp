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

//! The cross-check that keeps the parse-error cut policy *one* policy
//! (issue #1787).
//!
//! `tcl_lexer::first_parse_cut` answers the cut from source, for a compile
//! front-end that must know which commands run before the error. This crate
//! answers it per command, from the borrowed tree it has already built, for
//! an evaluator that must not substitute a word of a command that does not
//! parse. Two applications of one rule, deliberately not one call site: a
//! shared driver would make this crate re-lex every command it evaluates,
//! and its parse is infallible by design (errors ride as
//! `WordPart::ParseError`) where the owner's is a `Option<ParseCut>`.
//!
//! Two implementations of one rule drift. This is the test that says so.

use tcl_lexer::LexerConfig;
use tcl_runtime::parse::{first_parse_error, parse_script_with_config};

/// The cut this crate's per-command walk reports: the index of the first
/// command that fails to parse, and its message.
fn runtime_cut(src: &str, config: LexerConfig) -> Option<(usize, &'static str)> {
    parse_script_with_config(src.as_bytes(), config)
        .iter()
        .enumerate()
        .find_map(|(index, command)| {
            first_parse_error(&command.words, config).map(|message| (index, message))
        })
}

/// The cut the owner reports, in the same shape.
fn owner_cut(src: &str, config: LexerConfig) -> Option<(usize, &'static str)> {
    tcl_lexer::first_parse_cut(src, config).map(|cut| (cut.command, cut.message))
}

/// The malformed shapes, one per message C can raise, plus the two the
/// warning-stream scan the owner replaced answered wrongly and the one it
/// could not see at all.
const SHEET: &[&str] = &[
    "puts pre; puts \"x${abc\"",
    "puts pre; puts \"unterminated",
    "puts pre; puts [foo",
    "puts pre; set y {a}b",
    "puts pre; puts $a(",
    "puts pre; puts \"a\"b",
    "puts pre; set y {unclosed",
    // The two the flat warning stream got wrong.
    "puts pre; list [sfx one] [list \"oops]",
    "puts pre; puts $a([set q \"x)",
    // The one it could not see.
    "puts pre; sfx {a}{*}$b",
    // Nested a level down, where only a descent finds it.
    "puts pre; list [sfx one] [set y {a}b]",
    // Several clean commands before the cut.
    "puts one; puts two; puts \"x${abc\"",
    "sfx a; sfx b; puts $c(",
    // Clean.
    "puts pre; puts done",
    "puts \"a[foo \\\"b\\\"]c\"",
    "puts [list $ $x]",
    "lappend ev pre-[lindex $args end]",
];

/// The one shape the two engines still answer differently, pinned rather
/// than hidden: a word welded onto its **closing quote**.
///
/// C rejects `set y "a"b` with `extra characters after close-quote` on all
/// five shells; the owner reports it, and this crate silently concatenates
/// to `ab`. It is the exact sibling of the welded *close-brace* #1818
/// taught this crate to raise, and the boundary owner says so in as many
/// words — `WordSpan::welded_after_close` "is deliberately brace-only —
/// the analogous close-quote weld (`"a"b`) is a different C error and is
/// not reported here."
///
/// Raising a new error from the evaluator is a behaviour change with its
/// own oracle sheet, so it is a follow-up, not a rider on the owner. Until
/// then this row is the record that the divergence is known and measured,
/// not that the two engines agree.
const KNOWN_DIVERGENCES: &[&str] = &["puts pre; puts \"a\"b"];

#[test]
fn the_two_engines_agree_on_the_sheet() {
    for src in SHEET {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let config = LexerConfig::for_dialect(dialect);
            let (runtime, owner) = (runtime_cut(src, config), owner_cut(src, config));
            if KNOWN_DIVERGENCES.contains(src) {
                assert_ne!(
                    runtime, owner,
                    "{dialect}: {src:?} no longer diverges — \
                     drop it from KNOWN_DIVERGENCES"
                );
                continue;
            }
            assert_eq!(runtime, owner, "{dialect}: {src:?}");
        }
    }
}

/// The pinned divergence is exactly one shape, and it is the one described.
#[test]
fn the_close_quote_weld_is_the_only_pinned_divergence() {
    let config = LexerConfig::default();
    assert_eq!(
        KNOWN_DIVERGENCES
            .iter()
            .map(|src| (runtime_cut(src, config), owner_cut(src, config)))
            .collect::<Vec<_>>(),
        vec![(None, Some((1, "extra characters after close-quote")))],
    );
}

/// Real Tcl, where the two must agree that there is nothing to cut.
///
/// Walks every `.tcl` / `.test` / `.tm` under the corpora the boundary
/// differential already uses. A file the harness cannot read as UTF-8 is
/// skipped rather than failed — this is an agreement check, not an encoding
/// one.
#[test]
fn the_two_engines_agree_over_the_corpus() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root");
    let mut files = Vec::new();
    for corpus in ["samples", "tmp/tcl9.0.4/library", "tmp/tcllib-2.0/modules"] {
        collect(&root.join(corpus), &mut files);
    }
    assert!(
        files.len() > 100,
        "corpus is missing — found only {} file(s); run `make fetch-tcl-source`",
        files.len()
    );
    let config = LexerConfig::default();
    let mut disagreements = Vec::new();
    for file in &files {
        let Ok(src) = std::fs::read_to_string(file) else {
            continue;
        };
        let (runtime, owner) = (runtime_cut(&src, config), owner_cut(&src, config));
        // The pinned close-quote weld: `samples/tcl/05_warning_examples.tcl`
        // is a deliberate demonstration file and carries one.
        if runtime.is_none()
            && owner.is_some_and(|(_, m)| m == "extra characters after close-quote")
        {
            continue;
        }
        if runtime != owner {
            disagreements.push(format!(
                "{}: runtime {runtime:?} owner {owner:?}",
                file.display()
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} file(s) disagree:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "tcl" || ext == "test" || ext == "tm")
        {
            out.push(path);
        }
    }
}
