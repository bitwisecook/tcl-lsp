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

//! SSL-certificate tab: exercise the cert projection + cross-reference with an
//! inline config (the lab UCS fixtures terminate no TLS, so carry no certs).

use bigip_report_gen_rust::{Source, collect_model, collect_model_with_certs};
use serde_json::Value as J;

const SCF: &str = r#"
sys global-settings {
    hostname tls-test.example.net
}
sys file ssl-cert /Common/example.crt {
    cache-path /config/filestore/files_d/Common_d/certificate_d/:Common:example.crt_1_1
    certificate-key-size 2048
    expiration-date 1893456000
    expiration-string "Jan  1 00:00:00 2030 GMT"
    fingerprint "SHA256/AA:BB:CC"
    is-bundle false
    issuer "CN=Example Root CA,O=Example,C=US"
    key-size 2048
    key-type rsa-public
    serial-number 01:23:45
    source-path "file:///config/ssl/ssl.crt/example.crt"
    subject "CN=www.example.com,O=Example,C=US"
    subject-alternative-name "DNS:www.example.com, DNS:example.com"
    version 3
}
ltm profile client-ssl /Common/example_clientssl {
    cert /Common/example.crt
    ciphers DEFAULT
    defaults-from /Common/clientssl
    key /Common/example.key
}
ltm virtual /Common/web_https_vs {
    destination /Common/192.0.2.10:443
    ip-protocol tcp
    mask 255.255.255.255
    profiles {
        /Common/example_clientssl { }
        /Common/http { }
    }
    source 0.0.0.0/0
}
"#;

fn certs() -> Vec<J> {
    let sources: Vec<Source> = vec![("inline.scf".to_string(), SCF.to_string())];
    let m = collect_model(&sources, "TLS Test");
    m["devices"][0]["certificates"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn cert_projected_with_expiry_and_crossref() {
    let certs = certs();
    assert_eq!(certs.len(), 1, "one ssl-cert projected, got {certs:?}");
    let c = &certs[0];
    assert_eq!(c["name"], "example.crt");
    assert_eq!(c["fullPath"], "/Common/example.crt");
    assert!(
        c["subject"].as_str().unwrap().contains("www.example.com"),
        "subject: {c}"
    );
    assert!(c["issuer"].as_str().unwrap().contains("Example Root CA"));
    assert_eq!(c["expirationDate"], "1893456000");
    assert!(c["expirationString"].as_str().unwrap().contains("2030"));
    assert!(
        c["subjectAlternativeName"]
            .as_str()
            .unwrap()
            .contains("example.com")
    );
    // Cross-reference: the clientssl profile and the virtual that attaches it.
    let profs: Vec<&str> = c["usedByProfiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        profs.contains(&"example_clientssl"),
        "usedByProfiles: {profs:?}"
    );
    let virts: Vec<&str> = c["usedByVirtuals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(virts.contains(&"web_https_vs"), "usedByVirtuals: {virts:?}");
}

// --- f5mku secret decryption + private-key passphrase surfacing --------------

use bigip_report_gen_rust::decrypt_secrets;

// A config whose SSL key is passphrase-protected with an f5mku `$M$` secret.
// The known key/plaintext pair comes from tcl-f5mku's test vectors.
const F5MKU_KEY: &str = "BHDLd0bbao1VlwpTk1sioQ==";
const SCF_ENC: &str = r#"
sys file ssl-cert /Common/www.example.com.crt {
    expiration-date 1893456000
    expiration-string "Jan  1 00:00:00 2030 GMT"
    subject "CN=www.example.com"
}
sys file ssl-key /Common/www.example.com.key {
    security-type password
    passphrase $M$iP$rr0su9oHn9J9p1t3nRzydA==
}
ltm profile client-ssl /Common/www_clientssl {
    cert /Common/www.example.com.crt
    key /Common/www.example.com.key
}
ltm virtual /Common/www_https_vs {
    destination /Common/198.51.100.10:443
    profiles { /Common/www_clientssl { } }
}
"#;

fn first_cert(scf: &str) -> J {
    let m = collect_model(&[("inline.scf".to_string(), scf.to_string())], "TLS");
    m["devices"][0]["certificates"][0].clone()
}

#[test]
fn key_passphrase_encrypted_without_master_key() {
    let c = first_cert(SCF_ENC);
    assert!(c["hasKey"].as_bool().unwrap(), "cert paired with its key");
    assert!(
        c["keyPassphraseEncrypted"].as_bool().unwrap(),
        "passphrase still $M$"
    );
    assert!(c["keyPassphrase"].as_str().unwrap().starts_with("$M$"));
}

#[test]
fn f5mku_decrypts_key_passphrase() {
    let (scf, n) = decrypt_secrets(SCF_ENC, F5MKU_KEY).expect("decrypt ok");
    assert!(n >= 1, "at least one secret decrypted");
    let c = first_cert(&scf);
    assert_eq!(
        c["keyPassphrase"], "KEY45678",
        "passphrase revealed in clear"
    );
    assert!(!c["keyPassphraseEncrypted"].as_bool().unwrap());
}

#[test]
fn f5mku_wrong_key_errors() {
    let err = decrypt_secrets(SCF_ENC, "AAAAAAAAAAAAAAAAAAAAAA==").unwrap_err();
    assert!(
        err.to_string().contains("f5mku"),
        "clear wrong-key error: {err}"
    );
}

#[test]
fn empty_master_key_is_noop() {
    let (scf, n) = decrypt_secrets(SCF_ENC, "").expect("noop ok");
    assert_eq!(n, 0);
    assert!(scf.contains("$M$iP$"));
}

// --- Secrets tab -------------------------------------------------------------

use bigip_report_gen_rust::collect_secrets;

#[test]
fn secrets_inventory_lists_encrypted_and_decrypted() {
    // Without master key: the passphrase secret is listed as encrypted.
    let secs = collect_secrets(SCF_ENC);
    assert_eq!(secs.len(), 1, "one secret found: {secs:?}");
    let s = &secs[0];
    assert_eq!(s["field"], "passphrase");
    assert!(
        s["object"].as_str().unwrap().contains("ssl-key"),
        "object ctx: {}",
        s["object"]
    );
    assert!(s["encrypted"].as_bool().unwrap());

    // After decryption: same secret, now clear text.
    let (scf, _n) = decrypt_secrets(SCF_ENC, F5MKU_KEY).unwrap();
    let secs = collect_secrets(&scf);
    assert_eq!(secs.len(), 1);
    assert_eq!(secs[0]["value"], "KEY45678");
    assert!(!secs[0]["encrypted"].as_bool().unwrap());
}

#[test]
fn secrets_tab_present_only_when_secrets_exist() {
    let with = collect_model(&[("s.scf".to_string(), SCF_ENC.to_string())], "T");
    assert_eq!(with["devices"][0]["counts"]["secrets"], 1);
    // SCF (the plain cert-only config from the top of this file) has no secrets.
    let without = collect_model(&[("s.scf".to_string(), SCF.to_string())], "T");
    assert_eq!(without["devices"][0]["counts"]["secrets"], 0);
}

// --- Orphan proof: dynamic iRule attachment demotes to "possible" ------------

const SCF_ORPHAN: &str = r"
ltm pool /Common/unused_pool { members { /Common/10.0.0.9:80 { address 10.0.0.9 } } }
ltm pool /Common/used_pool { members { /Common/10.0.0.8:80 { address 10.0.0.8 } } }
ltm virtual /Common/vs1 { destination /Common/10.0.0.1:80 pool /Common/used_pool }
";

const SCF_ORPHAN_DYN: &str = r"
ltm pool /Common/unused_pool { members { /Common/10.0.0.9:80 { address 10.0.0.9 } } }
ltm pool /Common/used_pool { members { /Common/10.0.0.8:80 { address 10.0.0.8 } } }
ltm rule /Common/dyn { when HTTP_REQUEST { pool [class match -value [HTTP::host] equals hosts] } }
ltm virtual /Common/vs1 { destination /Common/10.0.0.1:80 pool /Common/used_pool rules { /Common/dyn } }
";

fn device0(scf: &str) -> J {
    collect_model(&[("o.scf".to_string(), scf.to_string())], "O")["devices"][0].clone()
}

#[test]
fn confirmed_orphan_without_dynamic_irule() {
    let d = device0(SCF_ORPHAN);
    let orphans: Vec<&str> = d["orphans"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        orphans.contains(&"unused_pool"),
        "unused_pool is a confirmed orphan: {orphans:?}"
    );
    assert_eq!(
        d["orphanRisk"].as_array().unwrap().len(),
        0,
        "no dynamic risk"
    );
}

#[test]
fn dynamic_irule_demotes_orphan_to_possible() {
    let d = device0(SCF_ORPHAN_DYN);
    // A dynamic `pool [class match …]` means no pool can be proven orphaned.
    assert!(
        d["orphans"]["pools"].as_array().unwrap().is_empty(),
        "no confirmed pool orphans"
    );
    let poss: Vec<&str> = d["possibleOrphans"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        poss.contains(&"unused_pool"),
        "unused_pool demoted to possible: {poss:?}"
    );
    let risk: Vec<&str> = d["orphanRisk"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        risk.contains(&"pools"),
        "pools flagged as dynamic-risk: {risk:?}"
    );
}

// A rule that dynamically attaches a *constrained* name (`web_[HTTP::host]`)
// only puts `web_`-prefixed pools in play — a `db_`-prefixed pool stays a
// confirmed orphan. This is the name-pattern filtering the deep analysis adds.
const SCF_ORPHAN_PREFIX: &str = r"
ltm pool /Common/web_backend { members { /Common/10.0.0.9:80 { address 10.0.0.9 } } }
ltm pool /Common/db_backend { members { /Common/10.0.0.7:80 { address 10.0.0.7 } } }
ltm pool /Common/used_pool { members { /Common/10.0.0.8:80 { address 10.0.0.8 } } }
ltm rule /Common/dyn { when HTTP_REQUEST { pool web_[HTTP::host] } }
ltm virtual /Common/vs1 { destination /Common/10.0.0.1:80 pool /Common/used_pool rules { /Common/dyn } }
";

#[test]
fn constrained_pattern_filters_orphans_by_name() {
    let d = device0(SCF_ORPHAN_PREFIX);
    let confirmed: Vec<&str> = d["orphans"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let possible: Vec<&str> = d["possibleOrphans"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    // `db_backend` cannot be built by `web_[HTTP::host]` — provably orphaned.
    assert!(
        confirmed.contains(&"db_backend"),
        "db_backend stays a confirmed orphan: confirmed={confirmed:?} possible={possible:?}"
    );
    // `web_backend` could be `web_<host>` — only a possible orphan.
    assert!(
        possible.contains(&"web_backend"),
        "web_backend is only a possible orphan: possible={possible:?}"
    );
    assert!(
        !confirmed.contains(&"web_backend"),
        "web_backend must not be a confirmed orphan"
    );
    // The reconstructed pattern is surfaced for the report UI.
    let pats = d["attachPatterns"]["pools"].as_array().unwrap();
    assert!(
        pats.iter().any(|p| p["glob"] == "web_*"),
        "web_* pattern surfaced: {pats:?}"
    );
    // And the possible orphan records which rule/pattern could reach it.
    let web = d["pools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "web_backend")
        .unwrap();
    assert_eq!(web["orphanStatus"], "possible");
    assert_eq!(web["orphanMatches"][0]["pattern"], "web_*");
}

// A `/Common` iRule attached only to a `/TenantA` virtual resolves its
// unqualified `pool web_…` in TenantA (with /Common visible). A pool in
// /TenantB is therefore provably unreachable by that rule.
const SCF_PARTITION: &str = r"
ltm pool /TenantA/web_a { members { /TenantA/10.0.0.9:80 { address 10.0.0.9 } } }
ltm pool /TenantB/web_b { members { /TenantB/10.0.0.7:80 { address 10.0.0.7 } } }
ltm rule /Common/dyn { when HTTP_REQUEST { pool web_[HTTP::host] } }
ltm virtual /TenantA/vsA { destination /TenantA/10.0.0.1:80 rules { /Common/dyn } }
";

#[test]
fn orphan_reachability_is_partition_aware() {
    let d = device0(SCF_PARTITION);
    let confirmed: Vec<&str> = d["orphans"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let possible: Vec<&str> = d["possibleOrphans"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    // Rule runs in TenantA -> web_a reachable (possible), web_b provably orphaned.
    assert!(
        possible.contains(&"web_a"),
        "web_a reachable in TenantA: possible={possible:?}"
    );
    assert!(
        confirmed.contains(&"web_b"),
        "web_b in TenantB unreachable by a TenantA-only rule: confirmed={confirmed:?}"
    );

    // The rule's per-partition table lists web_a under the TenantA group only.
    let rule = d["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "dyn")
        .unwrap();
    let groups = rule["referencedObjects"]["dynamic"].as_array().unwrap();
    let ta = groups
        .iter()
        .find(|g| g["partition"] == "TenantA")
        .expect("TenantA group present");
    let objs: Vec<&str> = ta["filters"][0]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap())
        .collect();
    assert!(
        objs.contains(&"web_a"),
        "TenantA filter matches web_a: {objs:?}"
    );
    assert!(
        !objs.contains(&"web_b"),
        "TenantA filter must not match web_b"
    );
    assert_eq!(ta["virtuals"][0], "vsA");
}

// A proc-library iRule (attached to no virtual, only invoked via
// `call Lib::proc`) is linked in as used by its caller, not flagged orphaned.
const SCF_PROC_CALL: &str = r"
ltm rule /Common/Lib { proc helper { x } { return $x } }
ltm rule /Common/Main { when HTTP_REQUEST { call Lib::helper 1 } }
ltm virtual /Common/vs1 { destination /Common/10.0.0.1:80 rules { /Common/Main } }
";

#[test]
fn cross_irule_proc_call_links_library() {
    let d = device0(SCF_PROC_CALL);
    let rules = d["rules"].as_array().unwrap();
    let main = rules.iter().find(|r| r["name"] == "Main").unwrap();
    let lib = rules.iter().find(|r| r["name"] == "Lib").unwrap();

    // Main references Lib via the proc call.
    let refs: Vec<&str> = main["refRules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(refs.contains(&"/Common/Lib"), "Main.refRules = {refs:?}");

    // The reference table lists the called iRule.
    let statics = main["referencedObjects"]["static"].as_array().unwrap();
    assert!(
        statics
            .iter()
            .any(|s| s["type"] == "irule" && s["name"] == "Lib"),
        "static refs include the called iRule: {statics:?}"
    );

    // Lib is used by Main -> not a confirmed orphan / not 'unattached'.
    let used: Vec<&str> = lib["usedBy"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(used.contains(&"/Common/Main"), "Lib.usedBy = {used:?}");
    let orphan_rules: Vec<&str> = d["orphans"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        !orphan_rules.contains(&"Lib"),
        "Lib not orphaned: {orphan_rules:?}"
    );
}

// Built-in / system objects (from the default config members) are tagged
// isDefault, excluded from counts, and never flagged as orphans.
const SCF_DEFAULTS: &str = "\
#
# config/profile_base.conf
#
ltm profile tcp tcp { }
ltm rule /Common/_sys_https_redirect { when HTTP_REQUEST { HTTP::respond 301 } }
#
# config/bigip.conf
#
ltm profile http /Common/app_http { }
ltm pool /Common/lonely_pool { members { /Common/10.0.0.9:80 { address 10.0.0.9 } } }
";

#[test]
fn default_objects_are_tagged_and_never_orphans() {
    let d = device0(SCF_DEFAULTS);
    let profiles = d["profiles"].as_array().unwrap();
    let tcp = profiles.iter().find(|p| p["name"] == "tcp").unwrap();
    let apphttp = profiles.iter().find(|p| p["name"] == "app_http").unwrap();
    assert_eq!(tcp["isDefault"], true, "default profile tagged");
    assert_eq!(apphttp["isDefault"], false, "user profile not tagged");

    // `_sys_*` rule is a default even though declared with a full path.
    let sys = d["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "_sys_https_redirect");
    assert!(
        sys.is_some_and(|r| r["isDefault"] == true),
        "_sys_ rule tagged"
    );

    // Counts exclude defaults: only the one user profile.
    assert_eq!(d["counts"]["profiles"], 1, "counts exclude defaults");

    // Defaults are never orphans; the user's unreferenced pool still is.
    let orphan_profiles: Vec<&str> = d["orphans"]["profiles"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_str().unwrap()).collect())
        .unwrap_or_default();
    assert!(!orphan_profiles.contains(&"tcp"), "default not orphaned");
    let orphan_rules: Vec<&str> = d["orphans"]["rules"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_str().unwrap()).collect())
        .unwrap_or_default();
    assert!(
        !orphan_rules.contains(&"_sys_https_redirect"),
        "_sys_ default not orphaned"
    );
}

// --- iRule syntax highlighting -----------------------------------------------

use bigip_report_gen_rust::highlight_tcl;

#[test]
fn irule_highlight_marks_commands_vars_events() {
    let html = highlight_tcl(
        "when HTTP_REQUEST {\n  set uri [HTTP::uri]\n  # note\n  log local0. $uri\n  pool /Common/p\n}",
    );
    assert!(
        html.contains(r#"<span class="tk-cmd">when</span>"#),
        "command: {html}"
    );
    assert!(
        html.contains(r#"<span class="tk-event">HTTP_REQUEST</span>"#),
        "event"
    );
    assert!(
        html.contains(r#"<span class="tk-ns">HTTP::uri</span>"#),
        "namespaced"
    );
    assert!(html.contains("tk-var"), "var: {html}");
    assert!(html.contains(r#"<span class="tk-comment">"#), "comment");
    // recursion into the event body highlighted the inner `pool`/`set` commands
    assert!(
        html.contains(r#"<span class="tk-cmd">set</span>"#),
        "inner command highlighted"
    );
    // model exposes bodyHtml
    let d = device0(SCF_ORPHAN_DYN);
    let rule = &d["rules"][0];
    assert!(
        rule["bodyHtml"].as_str().unwrap().contains("tk-"),
        "bodyHtml present"
    );
}

#[test]
fn irule_flowchart_is_graph_json() {
    let d = device0(SCF_ORPHAN_DYN);
    let fc = d["rules"][0]["flowchart"].as_str().unwrap();
    let g: serde_json::Value = serde_json::from_str(fc).expect("flowchart is JSON graph");
    let nodes = g["nodes"].as_array().expect("nodes array");
    assert!(
        nodes
            .iter()
            .any(|n| n["label"].as_str() == Some("HTTP_REQUEST")),
        "event node present: {fc}"
    );
    assert!(g["edges"].is_array(), "edges array present: {fc}");
}

// --- real certificate parsed out of a UCS filestore (parity with Python) -----

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../python/tests/data");

/// Load the chain fixture as a `(source, cert_pems)` pair the way the wasm /
/// CLI entry points do: flatten the UCS to SCF, then read each `sys file
/// ssl-cert` cache-path member back out of the archive.
fn chain_fixture() -> (Source, std::collections::HashMap<String, String>) {
    use tcl_bigip::model::ModelObject;
    use tcl_bigip::parser::driver::parse_bigip_conf;
    use tcl_bigip_io::{read_ucs_member, ucs_to_scf};

    let path = format!("{DATA}/certs-chain.ucs");
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let scf = ucs_to_scf(&raw, false).expect("extract");
    let mut pems = std::collections::HashMap::new();
    let config = parse_bigip_conf(&scf, "Common");
    for placed in &config.objects {
        if let ModelObject::SysFileSslCert(c) = &placed.object
            && !c.cache_path.is_empty()
            // The fixture is a plain (unencrypted) UCS, so the provider is never
            // invoked.
            && let Ok(pem) = read_ucs_member(
                &raw,
                &c.cache_path,
                &|| unreachable!("plain UCS needs no passphrase"),
                &path,
            )
        {
            pems.insert(
                c.cache_path.clone(),
                String::from_utf8_lossy(&pem).into_owned(),
            );
        }
    }
    ((path, scf), pems)
}

#[test]
fn cert_fields_and_chain_parsed_from_ucs_filestore() {
    let (source, pems) = chain_fixture();
    // The PEMs are scoped to their source URI (the collision-safe keying).
    let by_source = std::collections::HashMap::from([(source.0.clone(), pems)]);
    let m = collect_model_with_certs(&[source], "Chain", &by_source);
    let certs = m["devices"][0]["certificates"].as_array().unwrap();
    let leaf = certs
        .iter()
        .find(|c| c["name"] == "www.acme.example.crt")
        .expect("leaf cert present");

    // None of this is in the metadata-free stanza — it came from the PEM.
    assert_eq!(leaf["fromCertFile"], J::Bool(true));
    assert_eq!(leaf["cn"], "www.acme.example");
    assert!(
        leaf["issuer"]
            .as_str()
            .unwrap()
            .contains("Acme Intermediate CA")
    );
    assert!(
        !leaf["notBefore"].as_str().unwrap().is_empty(),
        "issue date"
    );
    assert!(
        leaf["expirationDate"]
            .as_str()
            .unwrap()
            .parse::<i64>()
            .is_ok()
    );
    let san = leaf["subjectAlternativeName"].as_str().unwrap();
    assert!(san.contains("api.acme.example") && san.contains("10.0.0.5"));
    assert!(!leaf["sigAlg"].as_str().unwrap().is_empty());

    let chain = leaf["chain"].as_array().unwrap();
    let roles: Vec<&str> = chain.iter().map(|l| l["role"].as_str().unwrap()).collect();
    assert_eq!(roles, ["leaf", "intermediate", "root"]);
    assert_eq!(chain.last().unwrap()["selfSigned"], J::Bool(true));
    assert_eq!(
        leaf["chainStatus"], "invalid",
        "the supplied signatures form a complete path, but the fixture leaf expired on 2026-08-13"
    );
    assert!(leaf["chainFindings"].as_array().is_some_and(|findings| {
        findings
            .iter()
            .any(|finding| finding["kind"] == "trust-unknown")
    }));
    assert!(
        leaf["chainFindings"].as_array().is_some_and(|findings| {
            findings.iter().any(|finding| finding["kind"] == "expired")
        })
    );
    assert!(
        m["devices"][0]["tls"]["trustDerCoverage"]["complete"]
            .as_u64()
            .is_some_and(|complete| complete > 100)
    );
}

// Two devices whose filestores use the SAME `cache-path` for DIFFERENT certs
// must each resolve to their own PEM. Before per-source keying, one flat
// `{cache_path: pem}` map meant the later device clobbered the earlier one and
// both cert tabs showed the same subject/chain.
#[test]
fn multi_ucs_shared_cache_path_does_not_collide() {
    // Two distinct real certs (different CNs) out of the chain fixture.
    let (_src, pems) = chain_fixture();
    let mut distinct: Vec<String> = pems.into_values().collect();
    assert!(distinct.len() >= 2, "chain fixture carries >= 2 certs");
    let pem_a = distinct.remove(0);
    let pem_b = distinct.remove(0);

    // Both devices declare an ssl-cert at the identical filestore cache-path.
    let shared = "/config/filestore/files_d/Common_d/certificate_d/:Common:shared.crt_1_1";
    let stanza = format!(
        "sys file ssl-cert /Common/shared.crt {{\n    cache-path {shared}\n    subject \"CN=stanza-placeholder\"\n}}\n"
    );
    let sources: Vec<Source> = vec![
        ("dev-a".to_string(), stanza.clone()),
        ("dev-b".to_string(), stanza.clone()),
    ];
    let by_source = std::collections::HashMap::from([
        (
            "dev-a".to_string(),
            std::collections::HashMap::from([(shared.to_string(), pem_a)]),
        ),
        (
            "dev-b".to_string(),
            std::collections::HashMap::from([(shared.to_string(), pem_b)]),
        ),
    ]);

    let m = collect_model_with_certs(&sources, "Multi", &by_source);
    let ca = &m["devices"][0]["certificates"][0];
    let cb = &m["devices"][1]["certificates"][0];
    // Each device parsed ITS OWN PEM (real subject, not the stanza placeholder).
    assert_eq!(ca["fromCertFile"], J::Bool(true), "dev-a parsed its PEM");
    assert_eq!(cb["fromCertFile"], J::Bool(true), "dev-b parsed its PEM");
    assert_ne!(
        ca["cn"], cb["cn"],
        "each device shows its own certificate, not a shared/collided one"
    );
}
