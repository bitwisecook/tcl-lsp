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

//! Authored package-index source advice under the shared written-name convention.

use super::{Availability, V86, V90, availabilities, only};

#[test]
fn authored_index_written_heads_keep_complete_colon_runs_and_literal_boundaries() {

    for head in ["package", "::package", ":::package", ":::::package"] {
        let source = format!("{head} ifneeded café 1.0 {{source café.tcl}}\n");
        assert_eq!(
            availabilities(&source, V86),
            vec![("café".to_owned(), Availability::Available)],
            "{head}",
        );
    }
    for head in [
        ":package",
        "ns:::package",
        "package:::n",
        "package:::",
        "$head",
        "${head}",
        "${head",
        ":::package[",
        "[set head package]",
        ":::Package",
    ] {
        let source = format!("{head} ifneeded café 1.0 {{source café.tcl}}\n");
        assert!(availabilities(&source, V86).is_empty(), "{head}");
    }
}

#[test]
fn authored_index_written_heads_keep_version_and_termination_conditions() {

    let source = ":::if {[:::package vsatisfies [:::::package provide Tcl] 9-]} {:::return}\n:::package ifneeded café 1.0 {source café.tcl}\n";
    assert_eq!(only(source, V86), Availability::Available);
    assert_eq!(only(source, V90), Availability::Unavailable);
    assert_eq!(only(source, None), Availability::Conditional);
    for source in [
        ":::return\n:::package ifneeded café 1.0 {source café.tcl}\n",
        ":::if {1} {:::::return}\n:::package ifneeded café 1.0 {source café.tcl}\n",
    ] {
        assert!(availabilities(source, V90).is_empty());
    }
    let loop_source =
        ":::foreach x {1} {:::::return}\n:::package ifneeded café 1.0 {source café.tcl}\n";
    assert_eq!(only(loop_source, V90), Availability::Conditional);
    for source in [
        ":::if {[:::package vsatisfies [:::package provide Tk] 9-]} {:::return}\n:::package ifneeded café 1.0 {source café.tcl}\n",
        ":::if {[:::package vsatisfies [:::package provide Tcl] $required]} {:::return}\n:::package ifneeded café 1.0 {source café.tcl}\n",
    ] {
        assert_eq!(only(source, V90), Availability::Conditional);
    }
}
