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

//! Integration tests for the Rust `f5report` port.
//!
//! Uses the same UCS fixtures as the Python `f5report` pytest suite
//! (`rust/bigip-report-gen/python/tests/data/`), so the two generators are validated
//! against identical inputs and the assertions mirror `test_report.py`.

// Assertions index into fixture-derived counts with `as` casts on values that
// are always small; truncation cannot occur for these test inputs.
#![allow(clippy::cast_possible_truncation)]

use bigip_report_gen_rust::{RenderOptions, Source, build_report, collect_model};
use serde_json::Value as J;
use tcl_bigip_io::ucs_to_scf;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../python/tests/data");

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
fn virtual_profiles_in_protocol_stack_order() {
    // `app1_t80_vs` lists its profiles `http` then `tcp` in the config, but a
    // BIG-IP processes the stack transport-first — the report must show TCP
    // ahead of HTTP, not the raw config (alphabetical) order.
    let m = model();
    let vs = arr(&m, "devices")
        .iter()
        .flat_map(|d| arr(d, "virtuals"))
        .find(|v| v["name"] == "app1_t80_vs")
        .expect("app1_t80_vs present");
    let profiles: Vec<&str> = vs["profiles"]
        .as_array()
        .expect("profiles array")
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    let tcp = profiles.iter().position(|p| p.ends_with("/tcp"));
    let http = profiles.iter().position(|p| p.ends_with("/http"));
    assert!(tcp < http, "expected tcp before http, got {profiles:?}");
}

#[test]
fn rule_events_in_firing_order() {
    // Events are emitted in canonical firing order, not alphabetically. For
    // every rule that has both, CLIENT_ACCEPTED must precede
    // CLIENTSSL_HANDSHAKE (which sorts the other way alphabetically).
    let m = model();
    for r in arr(&m, "devices").iter().flat_map(|d| arr(d, "rules")) {
        let evs: Vec<&str> = r["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap())
            .collect();
        if let (Some(a), Some(b)) = (
            evs.iter().position(|e| *e == "CLIENT_ACCEPTED"),
            evs.iter().position(|e| *e == "CLIENTSSL_HANDSHAKE"),
        ) {
            assert!(a < b, "firing order wrong in {evs:?}");
        }
    }
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
fn footer_shows_version_and_git_hash() {
    let sources = vec![load("lab-device-01.ucs")];
    let opts = RenderOptions {
        title: "Estate Report".into(),
        generated_at: "2026-07-03 00:00:00 UTC".into(),
        ..Default::default()
    };
    let html = build_report(&sources, &opts).expect("render");

    // The bottom bar carries a single `git describe --tags` version (v-tag +
    // commits + hash) and marks which backend rendered the report.
    assert!(html.contains("class=\"foot-build\""), "footer build line");
    assert!(
        html.contains(bigip_report_gen_rust::GIT_DESCRIBE),
        "git describe version in footer"
    );
    assert!(html.contains(">rust<"), "backend badge in footer");

    // The print running header (title) + footer (attribution/version/hash) are
    // emitted so the print stylesheet can repeat them on every page.
    assert!(
        html.contains("class=\"print-running-head\""),
        "print running header present"
    );
    assert!(
        html.contains("class=\"print-running-foot\""),
        "print running footer present"
    );
    // They are repeated on every printed sheet by being copied into the
    // thead/tfoot of the single-cell `table.print-sheet` the print run parks the
    // content in — the only construct a browser repeats per page (a position:
    // fixed box lands on the neighbouring page's margin instead).
    assert!(
        html.contains(".print-sheet-head") && html.contains(".print-sheet-foot"),
        "print stylesheet styles the per-page running head/foot"
    );
    assert!(
        html.contains("parkInSheet") || html.contains("print-sheet-body"),
        "print script parks the printed content in the running-head/foot sheet"
    );
}

#[test]
fn project_marks_inlined_as_svg_with_unique_ids() {
    let sources = vec![load("lab-device-01.ucs")];
    let opts = RenderOptions {
        title: "Marks".into(),
        generated_at: "2026-07-03 00:00:00 UTC".into(),
        ..Default::default()
    };
    let html = build_report(&sources, &opts).expect("render");

    // Both marks ride in the footer as real <svg>, not <img src="data:…">, and
    // each links somewhere useful.
    assert!(
        html.contains("class=\"foot-logo foot-logo--f5q\""),
        "f5q mark"
    );
    assert!(
        html.contains("class=\"foot-logo foot-logo--tcllsp\""),
        "tcl-lsp mark"
    );
    assert!(
        html.contains("docs/kcs/features/kcs-feature-bigip-query.md"),
        "f5q mark links to the f5-query quick start"
    );
    assert!(
        html.contains("href=\"https://github.com/bitwisecook/tcl-lsp\""),
        "tcl-lsp mark links to the repo root"
    );
    // tcl-lsp ships light + dark squircles; the report emits both and swaps on
    // the active theme.
    assert!(
        html.contains("logo-when-light") && html.contains("logo-when-dark"),
        "both tcl-lsp variants emitted"
    );
    // With no user-supplied logo the header falls back to the f5-query mark.
    assert!(
        html.contains("class=\"logo logo-mark\""),
        "default header mark is the f5q logo"
    );

    // Ids are document-global once the marks are inlined, and the minified SVGs
    // number their gradients `a`, `b`, `c`, … — so every logo id is namespaced
    // (`f5q-`, `hdrf5q-`, `tcl-`, `tcld-`). A duplicate would silently repaint
    // one mark with another's gradients; a dangling ref would drop the fill.
    let ids: Vec<&str> = html
        .match_indices(" id=\"")
        .map(|(i, m)| {
            let rest = &html[i + m.len()..];
            &rest[..rest.find('"').expect("closing quote")]
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        assert!(seen.insert(*id), "duplicate id in report: {id}");
    }
    // Every url(#…) the logos rely on must resolve to an id that is present.
    for (i, m) in html.match_indices("url(#") {
        let rest = &html[i + m.len()..];
        let target = &rest[..rest.find(')').expect("closing paren")];
        if target.starts_with("f5q-")
            || target.starts_with("hdrf5q-")
            || target.starts_with("tcl-")
            || target.starts_with("tcld-")
        {
            assert!(
                seen.contains(target),
                "dangling logo reference: url(#{target})"
            );
        }
    }
}

#[test]
fn forensics_tab_present_with_file_inventory() {
    // A file inventory (as the wasm/CLI entry points extract it) drives the
    // Forensics tab: the tab appears, and the checklist verdicts are embedded.
    let files = std::collections::HashMap::from([(
        "dev.ucs".to_string(),
        vec![
            serde_json::json!({
                "path": "root/.ssh/authorized_keys", "size": 24, "sha256": "a".repeat(64),
                "isText": true, "content": "ssh-rsa AAAAB3Nz attacker\n"
            }),
            serde_json::json!({
                "path": "etc/passwd", "size": 28, "sha256": "b".repeat(64),
                "isText": true, "content": "root:x:0:0::/root:/bin/bash\n"
            }),
        ],
    )]);
    let opts = RenderOptions {
        title: "Forensics".into(),
        generated_at: String::new(),
        embed_console: false,
        files,
        ..Default::default()
    };
    let sources = vec![(
        "dev.ucs".to_string(),
        "ltm pool /Common/p { }\n".to_string(),
    )];
    let html = build_report(&sources, &opts).expect("render");
    assert!(
        html.contains("data-panel=\"forensics\""),
        "forensics tab/panel present when files exist"
    );
    // The model (embedded JSON) carries the checklist; the non-empty
    // authorized_keys must be flagged.
    assert!(html.contains("ssh-authorized-keys"), "checklist embedded");
    assert!(
        html.contains("\\\"verdict\\\":\\\"alert\\\"") || html.contains("\"verdict\":\"alert\""),
        "authorized_keys flagged alert"
    );
}

#[test]
fn web_shell_irule_surfaces_forensics_tab_without_files() {
    // A config-only source (no UCS) whose iRule uses eval in an HTTP event:
    // the forensic file inventory is empty, but the flagged finding must still
    // surface the Forensics tab.
    let scf = "ltm rule /Common/shell { when HTTP_REQUEST { eval [b64decode [HTTP::header X-Cmd]] } }\n\
               ltm virtual /Common/vs { destination /Common/1.2.3.4:80 rules { /Common/shell } }\n";
    let opts = RenderOptions {
        title: "WebShell".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let html = build_report(&[("x.conf".to_string(), scf.to_string())], &opts).expect("render");
    assert!(
        html.contains("data-panel=\"forensics\""),
        "forensics tab present for a flagged iRule even with no files"
    );
    assert!(
        html.contains("irule-backdoor"),
        "the iRule finding is embedded"
    );
}

#[test]
fn no_forensics_tab_without_files() {
    // A bare bigip.conf source (no archive) → no forensic files → no tab.
    let opts = RenderOptions {
        title: "NoFx".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let sources = vec![("x.conf".to_string(), "ltm pool /Common/p { }\n".to_string())];
    let html = build_report(&sources, &opts).expect("render");
    assert!(
        !html.contains("data-panel=\"forensics\">"),
        "no forensics panel without a file inventory"
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

#[test]
fn relative_custom_profile_names_ordered_by_traffic() {
    // A virtual attaches custom profiles by partition-relative name; the
    // profile inventory is keyed by full path. Traffic ordering must still
    // resolve them (by leaf) so the transport profile leads the application
    // one, not the raw config order.
    let scf = "ltm virtual /Common/relvs {\n    destination /Common/1.2.3.4:80\n    profiles {\n        my_http { }\n        my_tcp { }\n    }\n}\nltm profile http /Common/my_http { }\nltm profile tcp /Common/my_tcp { }\n";
    let sources = vec![("mem://x".to_string(), scf.to_string())];
    let m = collect_model(&sources, "T");
    let vs = arr(&m, "devices")
        .iter()
        .flat_map(|d| arr(d, "virtuals"))
        .find(|v| v["name"] == "relvs")
        .expect("relvs present");
    let profs: Vec<&str> = vs["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(profs, vec!["my_tcp", "my_http"], "got {profs:?}");
}
