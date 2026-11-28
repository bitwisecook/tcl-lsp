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

use tcl_explorer::asm::serialise_asm;
use tcl_explorer::serialise::{
    serialise_cfg_post_ssa, serialise_gvn, serialise_irules_flow, serialise_shimmer,
    serialise_taint,
};
use tcl_explorer::{run_pipeline, serialise_result};
use tcl_lexer::LineIndex;

mod support;

#[test]
fn remaining_actual_cu_views_retain_profiles_and_ignore_display_labels() {
    support::install_surfaces();
    for engine in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
    ] {
        let mut result = run_pipeline("proc keep {v} {set copy $v; puts $copy}", engine);
        let current = serialise_result(&result);
        for field in [
            "asm",
            "wasm",
            "shimmer",
            "gvn",
            "taintWarnings",
            "taintReplays",
            "cfgPostSsa",
            "liveness",
            "irulesFlow",
            "bounds",
        ] {
            assert!(
                current[field].is_array() || current[field].is_object(),
                "{engine}/{field}: {}",
                current[field]
            );
        }
        result.dialect = "unrecognised-presentation-label".to_owned();
        let relabelled = serialise_result(&result);
        for field in [
            "asm",
            "wasm",
            "shimmer",
            "gvn",
            "taintWarnings",
            "taintReplays",
            "cfgPostSsa",
            "liveness",
            "irulesFlow",
            "bounds",
        ] {
            assert_eq!(relabelled[field], current[field], "{engine}/{field}");
        }
    }
}

#[test]
fn remaining_source_range_consumers_refuse_foreign_source_arguments() {
    support::install_surfaces();
    let result = run_pipeline("puts current", "tcl8.6");
    let source = "puts foreign";
    let li = LineIndex::new(source);
    for (field, value) in [
        ("asm", serialise_asm(&result, &li, source)),
        ("shimmer", serialise_shimmer(&result, &li, source)),
        ("gvn", serialise_gvn(&result, &li, source)),
        ("taintWarnings", serialise_taint(&result, &li, source)),
        ("cfgPostSsa", serialise_cfg_post_ssa(&result, &li, source)),
        ("irulesFlow", serialise_irules_flow(&result, &li, source)),
    ] {
        assert!(value.is_null(), "{field}");
    }
}

#[test]
fn retained_irules_warning_view_and_statistics_share_the_actual_count() {
    support::install_surfaces();
    let mut result = run_pipeline("when HTTP_REQUEST {set u [HTTP::uri]}", "f5-irules");
    let data = serialise_result(&result);
    let warnings = data["irulesFlow"]
        .as_array()
        .expect("available warning inventory");
    let count = warnings.len();
    assert!(count > 0, "fixture must produce a real warning");
    assert_eq!(data["stats"]["irulesFlowWarnings"], count);
    result.dialect = "tcl9.1".to_owned();
    let relabelled = serialise_result(&result);
    assert_eq!(relabelled["irulesFlow"], data["irulesFlow"]);
    assert_eq!(relabelled["stats"]["irulesFlowWarnings"], count);
}
