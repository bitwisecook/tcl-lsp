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
fn checked_census_distinguishes_empty_children_and_unavailable_source_phases() {
    let source = "set value 1";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let census = DispatchRegionCensus::capture(source, &analysis).unwrap();
    let mut observed_empty = false;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        &analysis,
        &mut |command, _, _| {
            assert_eq!(
                census.regions_for(source, &analysis, command),
                Some([].as_slice())
            );
            assert!(
                census
                    .regions_for("set value 2", &analysis, command)
                    .is_none()
            );
            let mut foreign = command.clone();
            foreign.all_tokens.clear();
            assert!(census.regions_for(source, &analysis, &foreign).is_none());
            observed_empty = true;
            false
        },
    );
    assert!(observed_empty);
    for source in ["if {1} $body", "apply $lambda", "$handler {}"] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            DispatchRegionCensus::capture(source, &analysis).is_none(),
            "{source}"
        );
    }
}
#[test]
fn complete_children_include_variable_index_and_expression_commands() {
    for source in ["set value $arr([set key 1])", "expr {[set key 1] + 2}"] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let census = DispatchRegionCensus::capture(source, &analysis).unwrap();
        let start = u32::try_from(source.find("set key 1").unwrap()).unwrap();
        let end = start + u32::try_from("set key 1".len()).unwrap();
        let mut outer = false;
        crate::executable_regions::visit_analysis_executable_commands(
            source,
            &analysis,
            &mut |command, _, _| {
                if command.span.start() == 0 {
                    let regions = census.regions_for(source, &analysis, command).unwrap();
                    assert!(
                        regions.contains(&(start as usize, end as usize)),
                        "{source}: {regions:?}"
                    );
                    outer = true;
                }
                false
            },
        );
        assert!(outer);
    }
}
