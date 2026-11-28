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

use tcl_explorer::{run_pipeline, serialise_result};

mod support;

#[test]
fn jim_pipeline_reports_unknown_calls_in_its_public_interprocedural_view() {
    support::install_surfaces();
    let data = serialise_result(&run_pipeline(
        "proc ::known {} {set local 1}\nproc ::unknown {} {nosuchcmd}\n",
        "jim",
    ));
    let rows = data["interprocedural"]
        .as_array()
        .expect("procedure summaries");
    let known = rows.iter().find(|row| row["name"] == "::known").unwrap();
    let unknown = rows.iter().find(|row| row["name"] == "::unknown").unwrap();
    assert_eq!(known["hasUnknownCalls"], false);
    assert_eq!(unknown["hasUnknownCalls"], true);
}

#[test]
fn document_pipeline_uses_the_requested_display_dialect() {
    support::install_surfaces();
    let result = run_pipeline("set value 1", "tcl8.6");
    assert_eq!(result.dialect, "tcl8.6");
    let data = serialise_result(&result);
    assert!(data["ir"].is_object());
    assert!(data["stats"]["functions"].is_u64());
}

#[test]
fn original_document_segments_keep_root_and_later_bom_roles() {
    support::install_surfaces();
    let source = "\u{feff}probe root\n\u{feff}probe later\n";
    for name in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let data = serialise_result(&run_pipeline(source, name));
        let rows = data["segments"].as_array().expect("source segments");
        let first = rows[0]["words"][0]["text"].as_str().unwrap();
        let grammar = tcl_registry::model::ingress::resolve_known_environment(name)
            .expect("known environment")
            .grammar();
        let expected = if grammar.script_skips_leading_bom {
            "probe"
        } else {
            "\u{feff}probe"
        };
        assert_eq!(first, expected, "{name}");
        assert_eq!(rows[1]["words"][0]["text"], "\u{feff}probe", "{name}");
    }
}

#[test]
fn changed_source_configuration_withdraws_structural_completeness() {
    support::install_surfaces();
    let source = "\u{feff}probe root";
    let mut result = run_pipeline(source, "tcl9.1");
    assert!(!serialise_result(&result)["structuralIndex"]["scriptComplete"].is_null());
    result.unit.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Content;
    assert!(!serialise_result(&result)["structuralIndex"]["scriptComplete"].is_null());
    result.source.push_str(" foreign");
    assert!(serialise_result(&result)["structuralIndex"]["scriptComplete"].is_null());
}

#[test]
fn snapshots_expose_procedure_and_body_unit_kinds() {
    support::install_surfaces();
    let result = run_pipeline(
        "::puts TOP; proc p {} {::puts PROC}; namespace eval n {::puts BODY}",
        "tcl8.6",
    );
    let snapshots = result.all_snapshots();
    assert!(snapshots.iter().any(|row| row.kind.as_str() == "procedure"));
    assert!(snapshots.iter().any(|row| row.kind.as_str() == "body-unit"));
    assert!(serialise_result(&result)["semantic"].is_array());
}

#[test]
fn shipped_profiles_drive_public_views_independently_of_display_labels() {
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
fn public_event_order_view_uses_the_pipeline_source() {
    support::install_surfaces();
    let data = serialise_result(&run_pipeline("when HTTP_REQUEST {}", "f5-irules"));
    let events = data["eventOrder"].as_array().expect("event order view");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "HTTP_REQUEST");
}

#[test]
fn retained_views_withdraw_after_the_result_source_changes() {
    support::install_surfaces();
    let mut result = run_pipeline("when HTTP_REQUEST {}", "f5-irules");
    assert!(serialise_result(&result)["eventOrder"].is_array());
    result.source.push_str("; set changed VALUE");
    let withdrawn = serialise_result(&result);
    assert!(withdrawn["eventOrder"].is_null());
    assert!(withdrawn["stats"].is_null());
}

#[test]
fn a_new_pipeline_image_serialises_its_own_source() {
    support::install_surfaces();
    let original = serialise_result(&run_pipeline("set retained ORIGINAL", "tcl8.4"));
    let rewritten = serialise_result(&run_pipeline("set retained REWRITTEN", "tcl8.4"));
    assert_ne!(original["ir"], rewritten["ir"]);
}

#[test]
fn bounds_view_withdraws_after_the_result_source_changes() {
    support::install_surfaces();
    let mut result = run_pipeline("proc keep {v} {puts $v}", "tcl8.6");
    assert!(serialise_result(&result)["bounds"].is_array());
    result.source.push_str(" foreign");
    assert!(serialise_result(&result)["bounds"].is_null());
}
