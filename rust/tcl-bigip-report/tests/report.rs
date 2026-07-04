//! Integration tests for the Rust `f5report` port.
//!
//! Uses the same UCS fixtures as the Python `f5report` pytest suite
//! (`rust/bigip-query-py/tests/data/`), so the two generators are validated
//! against identical inputs and the assertions mirror `test_report.py`.

#![allow(clippy::cast_possible_truncation)]

use serde_json::Value as J;
use tcl_bigip_io::ucs_to_scf;
use tcl_bigip_report::{RenderOptions, Source, build_report, collect_model};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../bigip-query-py/tests/data");

fn load(name: &str) -> Source {
    let path = format!("{DATA}/{name}");
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    // The lab fixtures are plain (unencrypted) UCS archives.
    let scf = ucs_to_scf(&raw, false).unwrap_or_else(|e| panic!("extract {path}: {e}"));
    (path, scf)
}

fn model() -> J {
    let sources = vec![load("lab-device-01.ucs"), load("lab-device-02.ucs")];
    collect_model(&sources, "Test Estate")
}

fn arr<'a>(v: &'a J, key: &str) -> &'a Vec<J> {
    v.get(key)
        .and_then(J::as_array)
        .unwrap_or_else(|| panic!("missing array {key}"))
}
fn u(v: &J, key: &str) -> u64 {
    v.get(key)
        .and_then(J::as_u64)
        .unwrap_or_else(|| panic!("missing uint {key}"))
}

#[test]
fn model_shape() {
    let m = model();
    assert_eq!(m["title"], "Test Estate");
    assert_eq!(arr(&m, "devices").len(), 2);
    let names: std::collections::BTreeSet<String> = arr(&m, "devices")
        .iter()
        .map(|d| d["name"].as_str().unwrap().to_string())
        .collect();
    let expected: std::collections::BTreeSet<String> =
        ["bigip-lab-01.example.net", "bigip-edge-02.example.net"]
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
    assert_eq!(names, expected);
}

#[test]
fn device_has_all_sections() {
    let m = model();
    let d = &arr(&m, "devices")[0];
    for key in [
        "virtuals",
        "pools",
        "nodes",
        "monitors",
        "rules",
        "dataGroups",
        "profiles",
    ] {
        assert!(
            d.get(key).is_some_and(J::is_array),
            "section {key} missing/not array"
        );
    }
    assert_eq!(u(&d["counts"], "virtuals"), arr(d, "virtuals").len() as u64);
}

#[test]
fn pool_members_and_usedby() {
    let m = model();
    let d = &arr(&m, "devices")[0];
    let pool = arr(d, "pools")
        .iter()
        .find(|p| p["name"] == "app1_t80_pool")
        .expect("app1_t80_pool present");
    let mc = u(pool, "memberCount") as usize;
    assert_eq!(mc, arr(pool, "members").len());
    assert!(mc >= 1);
    assert!(!pool["members"][0]["address"].as_str().unwrap().is_empty());
    assert!(
        arr(pool, "usedBy")
            .iter()
            .any(|x| x.as_str().unwrap().contains("app1_t443_vs"))
    );
}

#[test]
fn orphan_detection_consistent() {
    let m = model();
    let d = &arr(&m, "devices")[0];
    assert!(d["orphans"]["pools"].is_array());
    let total = u(&d["counts"], "orphans");
    let sum: u64 = d["orphans"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_array().unwrap().len() as u64)
        .sum();
    assert_eq!(total, sum);
}

#[test]
fn irule_events_extracted() {
    let m = model();
    let d = &arr(&m, "devices")[0];
    assert!(
        arr(d, "rules")
            .iter()
            .any(|r| !r["events"].as_array().unwrap().is_empty())
    );
}

#[test]
fn totals_sum_devices() {
    let m = model();
    let per_device: u64 = arr(&m, "devices")
        .iter()
        .map(|d| u(&d["counts"], "virtuals"))
        .sum();
    assert_eq!(u(&m["totals"], "virtuals"), per_device);
}

#[test]
fn build_report_html_self_contained() {
    let sources = vec![load("lab-device-01.ucs")];
    let opts = RenderOptions {
        title: "Solo".into(),
        generated_at: "2026-07-03 00:00:00 UTC".into(),
        embed_console: true,
        ..Default::default()
    };
    let html = build_report(&sources, &opts).expect("render");
    assert!(html.starts_with("<!doctype html>"), "doctype");
    assert!(html.contains("Solo"), "title present");
    // No unrendered minijinja delimiters (our template uses spaced `{{ x }}`).
    assert!(
        !html.contains("{{ ") && !html.contains(" }}") && !html.contains("{% "),
        "no raw template tags"
    );
    // Fully self-contained: no auto-loaded remote assets (scripts / images via
    // `src=`, stylesheets via `<link>`). Plain `<a href>` attribution links are
    // fine — they are user-initiated navigation, not loaded to run the report.
    assert!(!html.contains("src=\"http"), "no remote script/image src");
    assert!(!html.contains("<link "), "no external stylesheet links");
    // The wasm console + embedded config are present.
    assert!(html.contains("id=\"f5-wasm\""), "wasm blob embedded");
    assert!(html.contains("data-panel=\"console\""), "console panel");
    assert!(html.contains("wasm_bindgen"), "wasm glue");
    assert!(
        html.contains("\"configText\""),
        "config embedded for console"
    );
    // The SSL certificate tab is present.
    assert!(
        html.contains("data-panel=\"certificates\""),
        "certificates panel"
    );
}

#[test]
fn console_can_be_disabled() {
    let sources = vec![load("lab-device-01.ucs")];
    let opts = RenderOptions {
        title: "NoConsole".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let html = build_report(&sources, &opts).expect("render");
    // The console tab/panel elements (`…data-panel="console">`) are gone; a bare
    // `.panel[data-panel="console"]` CSS selector in the print stylesheet is not
    // the panel itself.
    assert!(
        !html.contains("data-panel=\"console\">"),
        "console tab/panel omitted"
    );
    assert!(!html.contains("id=\"f5-wasm\""), "no wasm blob");
    // Certificates tab still there (it does not depend on the console).
    assert!(html.contains("data-panel=\"certificates\""));
}

#[test]
fn model_json_serialisable() {
    let sources = vec![load("lab-device-01.ucs")];
    let m = collect_model(&sources, "F5 BIG-IP Configuration Report");
    serde_json::to_string(&m).expect("model serialises");
}
