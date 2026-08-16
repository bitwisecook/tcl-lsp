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
//!
//! # Why the model-shape assertions share one test
//!
//! [`model`] extracts two gzip'd `.ucs` archives and re-derives the whole
//! estate model. Nine tests used to assert nine independent facts about that
//! one value, and because nextest runs **one process per test**, no
//! `LazyLock`/`OnceLock` fixture can amortise the work across them — each
//! process paid the extraction again, 4-8 s apiece for ~120 s of a suite that
//! has one fixture in it.
//!
//! [`the_estate_model_has_the_shape_the_report_renders_from`] therefore derives
//! the model once and calls each of the shape checks below in turn. They stay
//! separate `fn`s named after what they pin, and every assertion carries a
//! message naming the property, so a failure still says which claim broke and
//! not merely "the model is wrong". The checks are pure reads of one immutable
//! value, so running them in one process cannot make one hide another.
//!
//! Tests of genuinely distinct behaviour — the HTML build, the console toggle,
//! the footer, the security/forensics tabs — keep their own `#[test]`, because
//! each drives a different render and a failure in one says nothing about the
//! others.

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
/// A JSON array's length as a `u64`, so it compares against a `counts` field
/// without a lossy cast.
fn len(v: &[J]) -> u64 {
    u64::try_from(v.len()).expect("a fixture array fits in u64")
}

/// Every pure statement this suite makes about the estate model, against one
/// derivation of it. See the module header for why they share a process.
#[test]
fn the_estate_model_has_the_shape_the_report_renders_from() {
    let m = model();
    model_shape(&m);
    device_has_all_sections(&m);
    pool_members_and_usedby(&m);
    orphan_detection_consistent(&m);
    irule_events_extracted(&m);
    virtual_profiles_in_protocol_stack_order(&m);
    rule_events_in_firing_order(&m);
    totals_sum_devices(&m);
    model_json_serialisable(&m);
}

fn model_shape(m: &J) {
    assert_eq!(
        m["title"], "Test Estate",
        "the estate title is carried through"
    );
    assert_eq!(
        arr(m, "devices").len(),
        2,
        "both source archives become devices"
    );
    let names: std::collections::BTreeSet<String> = arr(m, "devices")
        .iter()
        .map(|d| d["name"].as_str().unwrap().to_string())
        .collect();
    let expected: std::collections::BTreeSet<String> =
        ["bigip-lab-01.example.net", "bigip-edge-02.example.net"]
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
    assert_eq!(
        names, expected,
        "each device is named by its own hostname, not by its file"
    );
}

fn device_has_all_sections(m: &J) {
    let d = &arr(m, "devices")[0];
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
    assert_eq!(
        u(&d["counts"], "virtuals"),
        len(arr(d, "virtuals")),
        "the virtuals count matches the virtuals actually listed"
    );
}

fn pool_members_and_usedby(m: &J) {
    let d = &arr(m, "devices")[0];
    let pool = arr(d, "pools")
        .iter()
        .find(|p| p["name"] == "app1_t80_pool")
        .expect("app1_t80_pool present");
    let members = len(arr(pool, "members"));
    assert_eq!(
        u(pool, "memberCount"),
        members,
        "a pool's memberCount matches its member list"
    );
    assert!(members >= 1, "app1_t80_pool has members");
    assert!(
        !pool["members"][0]["address"].as_str().unwrap().is_empty(),
        "a pool member carries a resolved address"
    );
    assert!(
        arr(pool, "usedBy")
            .iter()
            .any(|x| x.as_str().unwrap().contains("app1_t443_vs")),
        "a pool names the virtuals that reference it"
    );
}

fn orphan_detection_consistent(m: &J) {
    let d = &arr(m, "devices")[0];
    assert!(
        d["orphans"]["pools"].is_array(),
        "orphans are reported per object kind"
    );
    let sum: u64 = d["orphans"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| len(v.as_array().unwrap()))
        .sum();
    assert_eq!(
        u(&d["counts"], "orphans"),
        sum,
        "the orphan count matches the orphans actually listed"
    );
}

fn irule_events_extracted(m: &J) {
    let d = &arr(m, "devices")[0];
    assert!(
        arr(d, "rules")
            .iter()
            .any(|r| !r["events"].as_array().unwrap().is_empty()),
        "at least one iRule has its events extracted"
    );
}

fn virtual_profiles_in_protocol_stack_order(m: &J) {
    // `app1_t80_vs` lists its profiles `http` then `tcp` in the config, but a
    // BIG-IP processes the stack transport-first — the report must show TCP
    // ahead of HTTP, not the raw config (alphabetical) order.
    let vs = arr(m, "devices")
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
    assert!(
        tcp < http,
        "a virtual's profiles are ordered transport-first: expected tcp before \
         http, got {profiles:?}"
    );
}

fn rule_events_in_firing_order(m: &J) {
    // Events are emitted in canonical firing order, not alphabetically. For
    // every rule that has both, CLIENT_ACCEPTED must precede
    // CLIENTSSL_HANDSHAKE (which sorts the other way alphabetically).
    for r in arr(m, "devices").iter().flat_map(|d| arr(d, "rules")) {
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
            assert!(
                a < b,
                "an iRule's events are in firing order, not alphabetical: {evs:?}"
            );
        }
    }
}

fn totals_sum_devices(m: &J) {
    let per_device: u64 = arr(m, "devices")
        .iter()
        .map(|d| u(&d["counts"], "virtuals"))
        .sum();
    assert_eq!(
        u(&m["totals"], "virtuals"),
        per_device,
        "the estate totals are the sum of the per-device counts"
    );
}

fn model_json_serialisable(m: &J) {
    serde_json::to_string(m).expect("the estate model serialises to JSON");
}

#[test]
fn tmsh_version_adds_k5903_release_lifecycle() {
    let sources = vec![(
        "mem://bigip-21.1.conf".to_owned(),
        "#TMSH-VERSION: 21.1.0.1\n".to_owned(),
    )];
    let m = collect_model(&sources, "Lifecycle");
    let lifecycle = &arr(&m, "devices")[0]["releaseLifecycle"];

    assert_eq!(lifecycle["branch"], "21.1.x");
    assert_eq!(lifecycle["releaseDate"], "2026-05-05");
    assert_eq!(lifecycle["eosdDate"], "2029-05-05");
    assert_eq!(lifecycle["eotsDate"], "2029-05-05");
    assert_eq!(lifecycle["eolDate"], "2029-05-05");
    assert_eq!(lifecycle["policyUpdated"], "2026-07-01");
    assert_eq!(
        lifecycle["sourceUrl"],
        "https://my.f5.com/manage/s/article/K5903"
    );
}

#[test]
fn lifecycle_panel_is_rendered_without_loading_remote_assets() {
    let sources = vec![(
        "mem://bigip-21.0.conf".to_owned(),
        "#TMSH-VERSION: 21.0.0\n".to_owned(),
    )];
    let opts = RenderOptions {
        title: "Lifecycle".into(),
        generated_at: "2026-08-02 00:00:00 UTC".into(),
        embed_console: false,
        ..Default::default()
    };
    let html = build_report(&sources, &opts).expect("render");

    assert!(html.contains("BIG-IP software lifecycle"));
    assert!(html.contains("EoSD"));
    assert!(html.contains("EoTS / EoL"));
    assert!(html.contains("Verify against F5 K5903"));
    assert!(!html.contains("src=\"https://my.f5.com"));
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
    // …and enforced in the browser: a CSP that forbids ALL network egress
    // (connect-src 'none'), so the config a report carries can never be phoned
    // home even if a future change or injected data tried to.
    assert!(
        html.contains("Content-Security-Policy") && html.contains("connect-src 'none'"),
        "report carries a no-network CSP"
    );
    // The wasm console + embedded config are present.
    assert!(html.contains("id=\"f5-wasm\""), "wasm blob embedded");
    // …and the blob must still be *decodable*. The template autoescapes by
    // default, which rewrites every `/` in the base64 as `&#x2f;`; `atob()` then
    // throws on load and the console, the iRule Format button and
    // print-with-diagnostics all die silently. Assert the payload is base64.
    let payload = html
        .split_once("id=\"f5-wasm\" type=\"application/octet-stream\">")
        .expect("wasm script tag")
        .1
        .split_once("</script>")
        .expect("closing tag")
        .0;
    assert!(
        !payload.is_empty()
            && payload
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=')),
        "wasm payload must be raw base64 — no HTML-escaped characters"
    );
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
fn tls_assurance_is_multi_sni_versioned_and_self_contained() {
    let source = r"#TMSH-VERSION: 17.1
ltm profile client-ssl /Common/base {
    defaults-from /Common/clientssl
    ciphers HIGH:!RSA
    options { no-tlsv1 no-tlsv1.1 }
}
ltm profile client-ssl /Common/sni {
    defaults-from /Common/base
    cert-key-chain {
        rsa { cert /Common/rsa.crt key /Common/rsa.key }
        ecdsa { cert /Common/ecdsa.crt key /Common/ecdsa.key }
    }
}
ltm virtual /Common/https {
    destination /Common/192.0.2.1:443
    profiles { /Common/sni { context clientside } }
}
";
    let sources = vec![("mem://tls.conf".to_owned(), source.to_owned())];
    let model = collect_model(&sources, "TLS assurance");
    let tls = &model["devices"][0]["tls"];
    assert_eq!(tls["endpoints"].as_array().map(Vec::len), Some(2));
    assert!(
        tls["endpoints"][0]["endpoint"]["protocols"]
            .as_array()
            .is_some_and(|protocols| protocols.iter().any(|value| value == "tls1.2"))
    );
    assert_eq!(tls["endpoints"][0]["estimate"]["grade"], "Unknown");
    assert_eq!(tls["endpoints"][0]["key_match"]["status"], "unknown");
    assert!(
        tls["endpoints"][0]["estimate"]["methodology"]
            .as_str()
            .is_some_and(|method| method.starts_with("sslictcl-"))
    );
    assert!(
        tls["provenance"]["sources"]
            .as_array()
            .is_some_and(|sources| !sources.is_empty())
    );
    assert!(
        tls["trustDerCoverage"]["total"]
            .as_u64()
            .is_some_and(|total| total > 100)
    );
    assert!(
        tls["trustCoverage"]["serverAuthPolicy"]["total"]
            .as_u64()
            .is_some_and(|total| total > 100)
    );
    assert!(
        tls["profileDefaultEvidence"]
            .as_array()
            .is_some_and(|evidence| evidence.iter().any(|item| item["state"] == "explicit"))
    );

    let html = build_report(
        &sources,
        &RenderOptions {
            title: "TLS assurance".to_owned(),
            generated_at: "2026-08-16 00:00:00 UTC".to_owned(),
            embed_console: false,
            ..RenderOptions::default()
        },
    )
    .expect("TLS report renders");
    assert_eq!(
        html.matches("data-panel=\"tls\"").count(),
        2,
        "one tab and one panel"
    );
    assert!(html.contains("TLS Assurance"));
    assert!(html.contains("Versioned security-default evidence"));
    assert!(html.contains("sslictcl-offline-estimate-v1"));
    assert!(html.contains("connect-src 'none'"));
    assert!(!html.contains("src=\"http"));
    assert!(!html.contains("<link "));
}

#[test]
fn configuration_diagnostics_render_in_their_object_tabs() {
    let source = "ltm pool /Common/empty {\n}\n\
                  ltm rule /Common/a {\n  when HTTP_REQUEST { one }\n}\n\
                  ltm rule /Common/b {\n  when HTTP_REQUEST { two }\n}\n\
                  ltm virtual /Common/vs {\n  rules { /Common/a /Common/b }\n}\n";
    let sources = vec![("test://diagnostics.conf".to_owned(), source.to_owned())];
    let html = build_report(&sources, &RenderOptions::default()).expect("render diagnostics");

    assert!(html.contains("Configuration diagnostics"));
    assert!(html.contains("BIGIP6008"), "pool diagnostic rendered");
    assert!(html.contains("BIGIP6012"), "virtual diagnostic rendered");
    let virtual_panel = html
        .split_once("<div class=\"panel active\" data-panel=\"virtuals\">")
        .expect("virtual panel")
        .1
        .split_once("<div class=\"panel\" data-panel=\"pools\">")
        .expect("pools panel")
        .0;
    assert!(virtual_panel.contains("BIGIP6012"));
}

#[test]
fn iapp_diagnostics_render_globally_and_in_the_apps_view() {
    let sources = vec![
        (
            "test://iapp/presentation.apl".to_owned(),
            "#include \"missing.inc\"\nsection basic {\n  string addr\n  string port\n}\n"
                .to_owned(),
        ),
        (
            "test://iapp/implementation.impl".to_owned(),
            "set ok $::basic__addr\nset missing $::basic__missing\n".to_owned(),
        ),
    ];
    let html = build_report(&sources, &RenderOptions::default()).expect("render iApp diagnostics");

    for code in ["IAPP7001", "IAPP7002", "IAPP7003"] {
        assert!(html.contains(code), "{code} rendered globally");
    }
    let apps_panels: Vec<_> = html
        .split("<div class=\"panel\" data-panel=\"apps\">")
        .skip(1)
        .map(|panel| {
            panel
                .split_once("<div class=\"panel\" data-panel=\"f5sites\">")
                .expect("F5 Sites panel")
                .0
        })
        .collect();
    assert!(apps_panels.iter().any(|panel| panel.contains("IAPP7001")));
    assert!(
        apps_panels
            .iter()
            .all(|panel| panel.contains("iApp diagnostic evidence"))
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
fn security_tab_flags_default_admin_credential() {
    // admin/admin, SHA-512-crypt — the flagship default-credential check.
    //
    // The *finding* itself never carries the stored hash/salt/candidate (see
    // `security::tests::no_secret_values_leak_into_the_model` for that
    // property, checked directly against `collect_security`'s own JSON). This
    // test can't repeat that assertion against the full rendered document: the
    // report deliberately re-embeds the entire raw source text verbatim
    // elsewhere (`configText`, for the in-browser query console) — same as it
    // always has for `secrets.rs`'s own values — so the hash legitimately
    // appears in the page regardless of what the Security tab does with it.
    let scf = "auth user /Common/admin {\n    encrypted-password $6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1\n}\n";
    let opts = RenderOptions {
        title: "Security".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let html = build_report(&[("x.conf".to_string(), scf.to_string())], &opts).expect("render");
    assert!(
        html.contains("data-panel=\"security\""),
        "security tab/panel present"
    );
    assert!(
        html.contains("BIGIP-SEC-001"),
        "default-credential rule id embedded"
    );
    assert!(
        html.contains("critical") && html.contains("confirmed"),
        "confirmed critical finding embedded"
    );
}

#[test]
fn security_tab_present_even_with_no_findings_to_confirm() {
    // A bare config with nothing to check still shows the tab (positive
    // assurance / documented limitation), never an alarming false positive.
    let opts = RenderOptions {
        title: "Security".into(),
        generated_at: String::new(),
        embed_console: false,
        ..Default::default()
    };
    let sources = vec![("x.conf".to_string(), "ltm pool /Common/p { }\n".to_string())];
    let html = build_report(&sources, &opts).expect("render");
    assert!(html.contains("data-panel=\"security\""));
    assert!(
        !html.contains("\"status\":\"confirmed\""),
        "nothing confirmed for a bare LTM-only config"
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

#[test]
fn profile_defaults_follow_the_bigip_version() {
    let report_profile = |version: &str| {
        let scf = format!(
            "#TMSH-VERSION: {version}\nltm profile client-ssl /Common/custom_clientssl {{ }}\n"
        );
        let model = collect_model(&[("mem://defaults".to_owned(), scf)], "T");
        arr(&model["devices"][0], "profiles")[0].clone()
    };

    let old = report_profile("20.1.0");
    assert_eq!(old["ciphers"], "DEFAULT");
    assert_eq!(old["effectiveFields"]["ciphers"], "DEFAULT");
    assert_eq!(old["defaultFields"]["cipher-group"], "none");

    let current = report_profile("21.1.0.1");
    assert_eq!(current["type"], "CLIENT_SSL");
    assert_eq!(current["ciphers"], "/Common/f5-default");
    assert_eq!(current["effectiveFields"]["ciphers"], "none");
    assert_eq!(
        current["defaultFields"]["cipher-group"],
        "/Common/f5-default"
    );
    assert_eq!(
        current["effectiveFields"]["options"],
        "dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl"
    );
}

#[test]
fn bigip_21_1_ai_defaults_and_mcp_persistence_reach_the_report() {
    let scf = "#TMSH-VERSION: 21.1.0.1\n\
               ltm profile aimcp /Common/ai { }\n\
               ltm profile json /Common/json_ai { }\n\
               ltm profile sse /Common/sse_ai { }\n\
               ltm persistence mcp /Common/mcp_ai { mcp-encryption-passphrase none }\n";
    let model = collect_model(&[("mem://ai".to_owned(), scf.to_owned())], "T");
    let device = &model["devices"][0];
    let profiles = arr(device, "profiles");

    let aimcp = profiles.iter().find(|p| p["name"] == "ai").unwrap();
    assert_eq!(aimcp["type"], "AIMCP");
    let json = profiles.iter().find(|p| p["name"] == "json_ai").unwrap();
    assert_eq!(json["type"], "JSON");
    assert_eq!(json["effectiveFields"]["maximum-entries"], "2048");
    let sse = profiles.iter().find(|p| p["name"] == "sse_ai").unwrap();
    assert_eq!(sse["type"], "SSE");
    assert_eq!(sse["effectiveFields"]["max-field-name-size"], "1024");

    let persistence = arr(device, "persistence")
        .iter()
        .find(|p| p["name"] == "mcp_ai")
        .unwrap();
    assert_eq!(persistence["fields"]["type"], "mcp");
    assert_eq!(persistence["fields"]["mcp-encryption-passphrase"], "none");
}
