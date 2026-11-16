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
fn original_receiver_hazards_cannot_be_narrowed_by_detached_source_class_reports() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create C {method method {} {}}; oo::class create Other {}; proc p {receiver} {$receiver method}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis
        .instance_classes
        .insert("receiver".to_owned(), "::Other".to_owned());
    let family = ["::C".to_owned()];
    let hazard = dispatch_hazard(
        source,
        analysis.resolved_profile().unwrap(),
        &analysis,
        MethodRenameTarget {
            family: &family,
            method: "method",
            is_classmethod: false,
            new_name: "changed",
        },
        &LineIndex::new(source),
    )
    .expect("unproved original receiver remains a hazard");
    assert!(hazard.reason.contains("class is not tracked"));
    let range = hazard.range.unwrap();
    assert_eq!(
        range.start_character,
        u32::try_from(source.rfind("method}").unwrap()).unwrap()
    );
}
