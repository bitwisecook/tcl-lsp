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
fn equal_width_changed_and_ambiguous_rows_keep_actual_evaluated_values() {
    let mut source = VerbatimIndex::of("speclib demo 1 {command demo {arity 1}}\n");
    assert!(source.row("arity", &["9".into()], 1).is_none());
    let captured = capture_stmt("arity", &["9".into()], 1);
    assert_eq!(captured.word_text(1), "9");
    let mut duplicate = VerbatimIndex::of("speclib demo 1 {command demo {arity 1; arity 2}}\n");
    assert!(duplicate.row("arity", &["1".into()], 1).is_none());
    let actual = source.row("arity", &["1".into()], 1).unwrap();
    assert_eq!(actual.word_text(1), "1");
    assert!(actual.arg(1).unwrap().original_source.is_some());
}

#[test]
fn one_word_original_body_is_available_but_changed_capture_cannot_reuse_it() {
    let mut source = VerbatimIndex::of("speclib demo 1 {command demo {pure}}\n");
    let body = source.take("command", "demo", 1).unwrap();
    let args = vec!["demo".to_owned(), "pure".to_owned()];
    assert!(
        body.original_for_capture("command", &args, "pure")
            .is_some()
    );
    let selected = BodyText::captured(Some(body), "command", &args, "pure".into(), 1);
    assert!(selected.verbatim);
    assert_eq!(selected.text, "pure");
    let mut source = VerbatimIndex::of("speclib demo 1 {command demo {pure}}\n");
    let body = source.take("command", "demo", 1).unwrap();
    let changed = vec!["demo".to_owned(), "arity 9".to_owned()];
    let selected = BodyText::captured(Some(body), "command", &changed, "arity 9".into(), 7);
    assert!(!selected.verbatim);
    assert!(selected.original.is_none());
    assert_eq!(selected.text, "arity 9");
    assert_eq!(selected.base, 7);
}
