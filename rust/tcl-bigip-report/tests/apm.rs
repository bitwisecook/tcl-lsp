//! Integration tests for the APM access-profile walk tab.
//!
//! Driven by a real APM config export (`tests/data/apm.scf`) — a per-session
//! access policy (logon page → AD auth → resource assign → allow/deny) with a
//! network-access resource, lease pool, webtops and remote-desktop resources.

use serde_json::Value as J;
use tcl_bigip_report::{RenderOptions, build_report, collect_model};

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
fn mermaid_is_vpe_shaped() {
    let m = collect_model(&[scf()], "APM");
    let mmd = m["devices"][0]["apmProfiles"][0]["mermaid"]
        .as_str()
        .expect("mermaid");
    // Left-to-right, orthogonal edges, rectangular nodes — like the VPE.
    assert!(mmd.contains("flowchart LR"));
    assert!(mmd.contains("stepAfter"));
    // The item flow, with branch captions on the connectors.
    assert!(mmd.contains("Logon Page"));
    assert!(mmd.contains("AD Auth"));
    assert!(mmd.contains("|Successful|") || mmd.contains("Successful"));
    // Green start/allow, red deny classes are applied.
    assert!(mmd.contains(":::start"));
    assert!(mmd.contains(":::allow"));
    assert!(mmd.contains(":::deny"));
    // Both remote-desktop resources are linked (inline word-list in the config).
    assert!(mmd.contains("JB-JC") && mmd.contains("JB-NC"));
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
    assert!(html.contains("flowchart LR"), "mermaid embedded");
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
