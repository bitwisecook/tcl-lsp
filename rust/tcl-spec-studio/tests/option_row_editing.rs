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

//! Pins the rule that lets an option row's text inputs be typed into.
//!
//! `options` is a `STRUCTURAL_KINDS` member, so every change to it used to
//! rebuild the whole field — `clear(ctl)` and recreate the DOM. That is right
//! for adding or removing a row, but wrong for a plain text edit: the input
//! the user is typing into is destroyed on the first keystroke, focus is lost,
//! and only one character lands. `STRUCTURAL_KINDS`' own doc comment warns
//! about exactly this ("a plain text or number input must not be [rebuilt], or
//! the caret jumps to the end mid-word"); composite kinds hold both sorts of
//! control, so the kind-level flag could not express it.
//!
//! Measured in Chromium, driving the real `options` editor and typing
//! `errorstack_value` one character at a time into the arity-hook box:
//!
//! ```text
//! rebuild on every change (old) : typed "e"                 — focus lost
//! rebuild only when structural  : typed "errorstack_value"  — focus held
//! ```
//!
//! The fix is a `structural` flag on `Setter`, defaulting to `true` so every
//! editor keeps its old behaviour. The option row passes `false` for scalar
//! edits and leaves it defaulted where a control appears or disappears.

const EDITORS_TS: &str = include_str!("../web/src/editors.ts");
const STUDIO_TS: &str = include_str!("../web/src/studio.ts");

/// Strip whitespace so an assertion survives reformatting.
fn squashed(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn the_setter_can_report_a_change_as_non_structural() {
    assert!(
        squashed(EDITORS_TS).contains("exporttypeSetter=(next:Json,structural?:boolean)=>void;"),
        "`Setter` must carry the optional `structural` flag — without it a \
         composite editor has no way to say that a text edit did not change \
         the editor's shape"
    );
}

#[test]
fn a_rebuild_only_happens_for_a_structural_change() {
    // The gate itself. `structural` defaults to true at the call site, so an
    // editor that never passes it behaves exactly as before.
    assert!(
        squashed(STUDIO_TS)
            .contains("if(structural&&STRUCTURAL_KINDS.has(field.kind.tag))rebuild();"),
        "the field rebuild must be gated on `structural`, or every keystroke \
         in an option row tears down the focused input"
    );
}

#[test]
fn option_row_text_edits_are_marked_non_structural() {
    let src = squashed(EDITORS_TS);
    // The option's own name and detail, the value's hint, and the arity hook —
    // every free-text box in the row.
    for (what, needle) in [
        ("option name", "patch({name:t},false)"),
        ("option detail", "patch({detail:t},false)"),
        ("value hint", "patchValue({hint:t},false)"),
    ] {
        assert!(
            src.contains(needle),
            "the {what} input must patch non-structurally ({needle} missing) \
             or it cannot be typed into continuously"
        );
    }
    assert!(
        src.contains(
            "patchValue({arity:{kind:\"Hook\",hook:t.trim()===\"\"?null:t.trim()}},false,)"
        ),
        "the arity-hook input must patch non-structurally too, or the hook \
         expression cannot be typed continuously"
    );
}

#[test]
fn switching_the_arity_kind_still_rebuilds() {
    // Toggling One/Fixed/Hook shows and hides the `n` and `hook fn` inputs, so
    // that one *must* rebuild — the flag is left defaulted.
    let src = squashed(EDITORS_TS);
    assert!(
        src.contains("patchValue({arity:{kind:\"Fixed\",n:asNumber(arity.n)??2}});"),
        "switching to a fixed count must rebuild so the `n` input appears"
    );
    assert!(
        src.contains("patchValue({arity:{kind:\"Hook\",hook:arity.hook??null}});"),
        "switching to a hook must rebuild so the `hook fn` input appears"
    );
}
