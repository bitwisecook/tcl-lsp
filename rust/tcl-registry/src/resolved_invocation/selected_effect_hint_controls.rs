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

#[test]
fn selected_descriptor_never_borrows_older_available_hint_rows() {
    let context = crate::model::ingress::resolve_environment("bpf").default_context_registry();
    let selected_hints = |name| {
        crate::model::assembly::resolve_invocation_in_context(
            context.commands(),
            Some(context.context()),
            name,
            &[],
        )
        .expect("actual selected descriptor")
        .selected_side_effect_hints()
        .len()
    };
    // `next` selects the bpf row, which declares no hints, while the walk
    // still finds the TclOO row's.
    assert_eq!(selected_hints("next"), 0);
    assert!(
        crate::model::assembly::side_effect_hints_in_context(
            context.commands(),
            Some(context.context()),
            "next",
            None,
        )
        .is_some()
    );
    // `classvariable` has no bpf row: its selected Tcl 9.0 row carries its
    // own hint, borrowed from nowhere.
    assert_eq!(selected_hints("classvariable"), 1);
    let context = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let selected = crate::model::assembly::resolve_invocation_in_context(
        context.commands(),
        Some(context.context()),
        "puts",
        &["message"],
    )
    .expect("actual selected I/O descriptor");
    assert_ne!(selected.selected_side_effect_hints().len(), 0);
}
