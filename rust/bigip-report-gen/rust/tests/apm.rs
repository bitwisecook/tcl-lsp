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

//! Integration tests for the APM access-profile walk tab.
//!
//! Driven by a real APM config export (`tests/data/apm.scf`) — a per-session
//! access policy (logon page → AD auth → resource assign → allow/deny) with a
//! network-access resource, lease pool, webtops and remote-desktop resources.

use bigip_report_gen_rust::{RenderOptions, build_report, collect_model};
use serde_json::Value as J;

fn scf() -> (String, String) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/apm.scf");
    (
        "apm.scf".to_string(),
        std::fs::read_to_string(path).expect("read apm.scf"),
    )
}

#[test]
fn model_carries_apm_walk() {
    let m = collect_model(&[scf()], "APM");
    let d = &m["devices"][0];
    assert_eq!(d["counts"]["apmProfiles"], J::from(1));
    let profiles = d["apmProfiles"].as_array().expect("apmProfiles array");
    assert_eq!(profiles.len(), 1);
    let p = &profiles[0];
    assert_eq!(p["name"], "mycave");
    assert_eq!(p["policy"], "mycave");
    // The access profile is attached by the mycave virtual server.
    assert!(
        p["virtuals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "mycave_vs"),
        "virtual linked back to profile"
    );
    // The walk reached every downstream object type.
    let types: std::collections::BTreeSet<&str> = p["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["type"].as_str())
        .collect();
    for want in [
        "ltm virtual",
        "apm profile access",
        "apm profile connectivity",
        "apm policy access-policy",
        "apm policy policy-item",
        "apm aaa",
        "apm resource network-access",
        "apm resource leasepool",
        "apm resource webtop",
        "apm resource remote-desktop",
    ] {
        assert!(types.contains(want), "walk missing object type {want}");
    }
}

#[test]
fn graph_model_is_vpe_shaped() {
    let m = collect_model(&[scf()], "APM");
    let graph = &m["devices"][0]["apmProfiles"][0]["graph"];
    let nodes = graph["nodes"].as_array().expect("nodes");
    let edges = graph["edges"].as_array().expect("edges");
    assert!(!nodes.is_empty() && !edges.is_empty());
    let labels: Vec<&str> = nodes.iter().filter_map(|n| n["label"].as_str()).collect();
    let classes: std::collections::BTreeSet<&str> =
        nodes.iter().filter_map(|n| n["cls"].as_str()).collect();
    let edge_labels: Vec<&str> = edges.iter().filter_map(|e| e["label"].as_str()).collect();
    // The item flow and its branch captions.
    assert!(labels.iter().any(|l| l.contains("Logon Page")));
    assert!(labels.iter().any(|l| l.contains("AD Auth")));
    assert!(edge_labels.contains(&"Successful"));
    // Green start/allow, red deny styling classes are applied.
    for cls in ["start", "allow", "deny"] {
        assert!(classes.contains(cls), "missing node class {cls}");
    }
    // Both remote-desktop resources are linked (inline word-list in the config).
    assert!(labels.iter().any(|l| l.contains("JB-JC")));
    assert!(labels.iter().any(|l| l.contains("JB-NC")));
}

#[test]
fn report_has_apm_tab() {
    let opts = RenderOptions {
        title: "APM".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let html = build_report(&[scf()], &opts).expect("render");
    assert!(html.contains("data-panel=\"apm\""), "APM tab/panel present");
    assert!(html.contains("apm-profile"), "profile card rendered");
    assert!(html.contains("apm-graph-data"), "graph model embedded");
    assert!(html.contains("ElkGraph"), "elkjs renderer embedded");
    // No unrendered template tags.
    assert!(
        !html.contains("{{ ") && !html.contains("{% "),
        "template fully rendered"
    );
}

#[test]
fn no_apm_tab_without_apm_objects() {
    // An LTM-only config must not grow an APM tab.
    let ltm = "ltm virtual /Common/v { destination /Common/1.2.3.4:80 }\n";
    let m = collect_model(&[("x".into(), ltm.into())], "LTM");
    assert_eq!(m["devices"][0]["counts"]["apmProfiles"], J::from(0));
    let opts = RenderOptions {
        title: "LTM".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let html = build_report(&[("x".into(), ltm.into())], &opts).expect("render");
    assert!(
        !html.contains("data-panel=\"apm\">"),
        "no APM tab for LTM-only config"
    );
}
