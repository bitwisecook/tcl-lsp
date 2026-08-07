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

//! Issue #1305 — a `rename`d metaclass command manufactures nothing.
//!
//! `rename ::R::M ::R::Mk` then `::R::Mk create ::R::W {…}` used to abstain:
//! the class-factory record was keyed on the name the metaclass was
//! *created* with, and nothing carried it across the `rename`.
//!
//! C-Tcl ground truth (tclsh 8.6.16 / 9.0.4): `rename` moves the class's
//! command, and the class keeps working through its new name, so
//! `::R::W`'s `go` is a real, callable method.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

const SRC: &str = concat!(
    "namespace eval ::R {}\n",
    "oo::class create ::R::M { superclass oo::class }\n",
    "rename ::R::M ::R::Mk\n",
    "::R::Mk create ::R::W { method go {} { return went } }\n",
    "set w [::R::W new]\n",
    "puts [$w go]\n",
);

/// TP — the ticket. Go-to-definition on `go` in `puts [$w go]` resolves to
/// the method's declaration, only reachable if the renamed-through creation
/// call was recognised as a factory call.
#[test]
fn go_to_definition_resolves_through_the_renamed_metaclass() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SRC);
    // `puts [$w go]` is line 5 (0-based); `go` starts at character 10.
    let found = locations(&lsp.definition(&uri, 5, 10));
    assert_eq!(
        found.len(),
        1,
        "renamed-metaclass creation must still manufacture a navigable class: {found:?}"
    );
}

/// TP control — the same shape without the `rename` (the metaclass created
/// directly under the name it is called through) resolves identically, so
/// the renamed case above is judged against its own oracle.
#[test]
fn go_to_definition_resolves_without_a_rename() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = concat!(
        "namespace eval ::R {}\n",
        "oo::class create ::R::Mk { superclass oo::class }\n",
        "::R::Mk create ::R::W { method go {} { return went } }\n",
        "set w [::R::W new]\n",
        "puts [$w go]\n",
    );
    lsp.open_ready(&uri, src);
    // `puts [$w go]` is line 4 (0-based) in this shorter source.
    let found = locations(&lsp.definition(&uri, 4, 10));
    assert_eq!(found.len(), 1, "{found:?}");
}
