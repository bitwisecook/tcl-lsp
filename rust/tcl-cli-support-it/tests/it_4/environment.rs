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

use tcl_cli_support::environment::{context_for_dialect, profile_for_dialect};
use tcl_dialect::DialectProfile;

/// The document context a verb answers under asks the same surface
/// questions the threaded profile does, for every name this ingress can
/// produce.
///
/// Not structural equality: the context resolves the F5 trunk to its
/// measured build (`8.4.6`) where the profile names the ladder line
/// (`8.4`), and no shipped surface tells those apart.
#[test]
fn the_document_context_answers_like_the_threaded_profile() {
    crate::install_surfaces();
    let names: Vec<&str> = DialectProfile::all()
        .iter()
        .map(|profile| profile.name)
        .chain(["tk", "tcl", "", "irules", "not-a-real-dialect"])
        .collect();
    for name in names {
        assert!(
            tcl_registry::model::surface::points_answer_alike(
                &context_for_dialect(name).authoring_query(),
                &profile_for_dialect(name).surface_query(),
            ),
            "{name}"
        );
    }
}
