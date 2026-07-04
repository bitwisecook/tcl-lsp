//! SSL-certificate tab: exercise the cert projection + cross-reference with an
//! inline config (the lab UCS fixtures terminate no TLS, so carry no certs).

use serde_json::Value as J;
use tcl_bigip_report::{Source, collect_model};

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

use tcl_bigip_report::decrypt_secrets;

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

use tcl_bigip_report::collect_secrets;

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
    assert!(objs.contains(&"web_a"), "TenantA filter matches web_a: {objs:?}");
    assert!(!objs.contains(&"web_b"), "TenantA filter must not match web_b");
    assert_eq!(ta["virtuals"][0], "vsA");
}

// --- iRule syntax highlighting -----------------------------------------------

use tcl_bigip_report::highlight_tcl;

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
fn irule_flowchart_is_mermaid() {
    let d = device0(SCF_ORPHAN_DYN);
    let fc = d["rules"][0]["flowchart"].as_str().unwrap();
    assert!(fc.starts_with("flowchart TD"), "mermaid flowchart: {fc}");
    assert!(fc.contains("HTTP_REQUEST"), "event node present");
}
