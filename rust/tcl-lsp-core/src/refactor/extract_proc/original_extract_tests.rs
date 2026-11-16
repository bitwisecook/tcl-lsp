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
use tcl_compiler::analyser::Analyser;

#[test]
fn original_extract_name_collision_uses_publications_without_reporting_maps() {
    // Implementation contract: naming.refactor.original-extracted-procedure-publication
    // docs/design/analysis/name-resolution-proofs/refactor-original-extracted-procedure-publication.md
    let source = "proc extracted_proc {} {}; interp alias {} extracted_proc_2 {} list; puts VALUE";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    assert_eq!(
        unique_proc_name(source, &analysis, analysis.resolved_registry().unwrap()).unwrap(),
        "extracted_proc_3"
    );
    assert!(
        unique_proc_name(
            &format!("#{source}"),
            &analysis,
            analysis.resolved_registry().unwrap()
        )
        .is_err()
    );
    assert!(
        extract_proc(
            &format!("#{source}"),
            (1, 20),
            &analysis,
            analysis.resolved_registry().unwrap()
        )
        .is_none()
    );
}

#[test]
fn original_extract_name_search_refuses_unknown_or_fully_occupied_slots() {
    // Implementation contract: naming.refactor.original-extracted-procedure-publication
    // docs/design/analysis/name-resolution-proofs/refactor-original-extracted-procedure-publication.md
    assert!(first_fresh_proc_name(|_| None).is_err());
    assert!(first_fresh_proc_name(|_| Some(true)).is_err());
    assert_eq!(
        first_fresh_proc_name(|name| Some(name != "extracted_proc_3")).unwrap(),
        "extracted_proc_3"
    );
}
