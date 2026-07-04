//! Integration tests for the multi-device *architecture* layer.
//!
//! Uses small inline SCF sources (the report engine takes `(uri, scf_text)`
//! directly, so no UCS fixture is needed) to exercise cross-device link
//! auto-detection and manifest overrides.

use std::collections::HashMap;

use serde_json::Value as J;
use tcl_bigip_report::{Source, collect_model, collect_model_with_architecture};

/// Empty cert-PEM map for the architecture-only tests (source-scoped:
/// `uri -> {cache_path -> pem}`).
fn no_certs() -> HashMap<String, HashMap<String, String>> {
    HashMap::new()
}

/// A tier-1 LTM: a virtual on 10.1.0.10 whose pool member is 10.2.0.20 — the
/// address the tier-2 device serves. That IP overlap is the tier hop.
const TIER1: &str = r"#TMSH-VERSION: 15.1.0
ltm virtual /Common/edge_vs {
    destination /Common/10.1.0.10:443
    pool /Common/edge_pool
}
ltm pool /Common/edge_pool {
    members {
        /Common/10.2.0.20:443 {
            address 10.2.0.20
        }
    }
}
ltm virtual-address /Common/10.1.0.10 {
    address 10.1.0.10
}
";

/// A tier-2 LTM: a virtual on 10.2.0.20 (the tier-1 pool member) fronting a
/// backend on 10.3.0.30.
const TIER2: &str = r"#TMSH-VERSION: 15.1.0
ltm virtual /Common/app_vs {
    destination /Common/10.2.0.20:443
    pool /Common/app_pool
}
ltm pool /Common/app_pool {
    members {
        /Common/10.3.0.30:8080 {
            address 10.3.0.30
        }
    }
}
ltm virtual-address /Common/10.2.0.20 {
    address 10.2.0.20
}
";

fn sources() -> Vec<Source> {
    vec![
        ("tier1.scf".to_string(), TIER1.to_string()),
        ("tier2.scf".to_string(), TIER2.to_string()),
    ]
}

fn arch(model: &J) -> &J {
    model.get("architecture").expect("architecture present")
}

#[test]
fn architecture_is_always_present() {
    let m = collect_model(&sources(), "Estate");
    let a = arch(&m);
    assert_eq!(a["defined"], J::Bool(false), "no manifest -> auto only");
    assert_eq!(a["deviceCount"], J::from(2));
    assert_eq!(a["devices"].as_array().unwrap().len(), 2);
}

#[test]
fn auto_detects_tier_hop() {
    let m = collect_model(&sources(), "Estate");
    let a = arch(&m);
    let links = a["links"].as_array().unwrap();
    assert_eq!(links.len(), 1, "one tier-1 -> tier-2 link");
    let l = &links[0];
    assert_eq!(l["from"], J::from(0));
    assert_eq!(l["to"], J::from(1));
    assert_eq!(l["source"], "auto");
    // The evidencing flow is the pool member -> virtual on 10.2.0.20.
    let via = &l["vias"].as_array().unwrap()[0];
    assert_eq!(via["address"], "10.2.0.20");
    assert_eq!(via["fromObj"], "/Common/edge_pool");
    assert_eq!(via["toObj"], "/Common/app_vs");
}

#[test]
fn pool_member_without_address_uses_name() {
    // A tier-1 pool member declared by keyed name only (no `address` property);
    // the IP lives in the name. The hop must still be detected.
    let tier1 = "#TMSH-VERSION: 15.1.0
ltm pool /Common/edge_pool {
    members {
        /Common/10.2.0.20:443 { }
    }
}
";
    let tier2 = "#TMSH-VERSION: 15.1.0
ltm virtual /Common/app_vs {
    destination /Common/10.2.0.20:443
}
ltm virtual-address /Common/10.2.0.20 {
    address 10.2.0.20
}
";
    let sources: Vec<Source> = vec![
        ("t1.scf".to_string(), tier1.to_string()),
        ("t2.scf".to_string(), tier2.to_string()),
    ];
    let m = collect_model(&sources, "Estate");
    let links = arch(&m)["links"].as_array().unwrap();
    assert_eq!(links.len(), 1, "tier hop detected from the member name");
    assert_eq!(links[0]["from"], 0);
    assert_eq!(links[0]["to"], 1);
    assert_eq!(links[0]["vias"].as_array().unwrap()[0]["address"], "10.2.0.20");
}

#[test]
fn tiers_are_assigned_by_depth() {
    let m = collect_model(&sources(), "Estate");
    let devs = arch(&m)["devices"].as_array().unwrap();
    let tier = |i: usize| devs[i]["tier"].as_i64().unwrap();
    assert_eq!(tier(0), 0, "the fronting device is tier 0");
    assert_eq!(tier(1), 1, "the device it fronts is tier 1");
}

#[test]
fn tcl_manifest_overrides_and_links() {
    // The manifest is a small Tcl script parsed with the real Tcl tokeniser.
    let manifest = r#"
        # architecture of this estate
        device tier1.scf -role gtm -tier 5 -label "DNS Front"
        device tier2.scf -role ltm -label "App Tier"

        link tier2.scf tier1.scf -label mgmt
    "#;
    let m = collect_model_with_architecture(&sources(), "Estate", &no_certs(), Some(manifest));
    let a = arch(&m);
    assert_eq!(a["defined"], J::Bool(true));
    let devs = a["devices"].as_array().unwrap();
    assert_eq!(devs[0]["role"], "gtm");
    assert_eq!(devs[0]["tier"], J::from(5));
    assert_eq!(devs[0]["label"], "DNS Front", "quoted multi-word label parsed");
    assert_eq!(devs[1]["label"], "App Tier");

    // Auto 0->1 plus the manifest-declared 1->0.
    let links = a["links"].as_array().unwrap();
    assert_eq!(links.len(), 2);
    assert!(links.iter().any(|l| l["from"] == 1
        && l["to"] == 0
        && l["source"] == "manifest"
        && l["label"] == "mgmt"));
}

#[test]
fn malformed_manifest_is_reported_not_fatal() {
    // An unbalanced brace makes the Tcl tokeniser fail.
    let m = collect_model_with_architecture(
        &sources(),
        "Estate",
        &no_certs(),
        Some("device tier1.scf -label {unclosed"),
    );
    let a = arch(&m);
    assert!(a.get("manifestError").is_some(), "tokeniser error surfaced");
    // Auto-detection still ran.
    assert_eq!(a["links"].as_array().unwrap().len(), 1);
}
