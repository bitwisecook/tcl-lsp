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

//! A published pack set's roster for Jim's pair replaces the compiled-in one.
//!
//! The roster store is process-wide, so this does not share a process with
//! the `jim` harness, whose tests read the compiled-in roster.

use std::path::PathBuf;

use tcl_dialect::model::{Family, roster_for};
use tcl_spectcl::discovery::{Origin, PackFile, Tier};

/// A head Tcl 8.6 has and no `jimsh` does, so the compiled-in roster
/// withholds it and a roster that admits it is a visible difference.
const NOT_IN_JIM: &str = "coroutine";

/// A head `jimsh` has, which the compiled-in roster admits and the roster
/// published below does not list.
const ONLY_IN_THE_COMPILED_IN_ROSTER: &str = "proc";

fn jim_admits(head: &str) -> bool {
    roster_for(Family::Jim, Family::Tcl)
        .expect("a roster is registered for jim")
        .admits(head, None)
}

/// The publication's roster replaces Jim's compiled-in one for the same pair.
#[test]
fn a_publication_replaces_the_compiled_in_roster_for_its_pair() {
    let pack = PackFile {
        tier: Tier::Bundled,
        path: PathBuf::from("restated.tclspec"),
        origin: Origin::Bundled,
        dependency_tier: None,
    };
    let source =
        format!("speclib restated 2.0 {{\n include from tcl into jim {{set {NOT_IN_JIM}}}\n}}");
    let set = tcl_spectcl::pack::load_in_memory(vec![(pack, source)]);

    let _ = tcl_spectcl::publish_pack_set(&set);
    assert!(jim_admits(NOT_IN_JIM), "the published roster admits it");
    assert!(!jim_admits(ONLY_IN_THE_COMPILED_IN_ROSTER));
}
