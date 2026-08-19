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

//! `bigip-data-schema` — the BIG-IP object-spec data consistency gate
//! (issue #1404 item 2).
//!
//! `rust/tcl-registry/src/bigip/data/{a,c,g,i,l,m,n,p,s,u,v,w}.rs` (798
//! object kinds, bucketed by the first letter of `kind`) carried a
//! `Generated ... DO NOT EDIT` header naming a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists in this branch's tree, and its own history — the commits that
//! actually ran it against the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) — was
//! intentionally squashed away by the `rust` branch's rebase-onto-main
//! commit. There is nothing left in this branch's git history to replay,
//! so the header now says what these files actually are: hand-maintained.
//!
//! What a generator would otherwise have guaranteed for free is instead
//! enforced here as a structural drift gate:
//!
//! - **uniqueness** — no `kind` name appears in more than one spec;
//! - **filing** — every spec lives in the bucket file matching the first
//!   letter of its `kind` (a spec pasted into the wrong letter file, or a
//!   renamed kind left in its old bucket, both show up here);
//! - **scan/registry agreement** — the source-text scan this module does
//!   (regex over `kind: "..."` — the per-letter data modules are private to
//!   `tcl-registry`, so an xtask outside that crate cannot enumerate them
//!   per-file through the public API) must find exactly the same kind set
//!   the compiled registry ([`tcl_registry::bigip::data::all_specs`])
//!   reports, catching both a scan/format mismatch and a `data/mod.rs`
//!   wiring gap (a bucket module added but never aggregated into
//!   `all_specs`, mirrored here as never added to `BUCKETS`);
//! - **reference integrity** — every `BigipPropertySpec::references` entry
//!   either names a real `kind`, or is on [`KNOWN_UNRESOLVED_REFERENCES`],
//!   the documented pre-existing gap list discovered when this gate was
//!   introduced (tmsh action verbs like `start`/`stop`/`save` captured
//!   alongside real object-kind references, and object kinds the dataset
//!   never carried a spec for). A *new* unresolved reference — not on that
//!   list — fails the gate; entries can be researched and removed from the
//!   list over time as they're confirmed real or fixed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

/// The letter-bucket data files, relative to the repo root — must match
/// `rust/tcl-registry/src/bigip/data/mod.rs`'s `mod` list exactly (that
/// agreement is itself checked: see the module doc's "scan/registry
/// agreement" bullet).
const BUCKETS: &[&str] = &["a", "c", "g", "i", "l", "m", "n", "p", "s", "u", "v", "w"];

const DATA_DIR: &str = "rust/tcl-registry/src/bigip/data";

/// Reference targets already unresolved when this gate was introduced —
/// tmsh action verbs (`start`, `stop`, `save`, `restart`, `load`) that share
/// the `references` field with real object-kind references, and object
/// kinds the 798-kind dataset never carried a spec for. Not re-researched
/// here (no F5 documentation access from this checkout); each entry is a
/// candidate for either a real fix (add the missing kind, or split action
/// verbs into their own field) or removal once confirmed intentional.
/// A name that stops being unresolved (because the spec appeared, or the
/// reference was fixed) must be removed from this list — `--check` fails
/// on a stale entry the same as it fails on a new gap, so the list cannot
/// silently grow stale in the other direction.
const KNOWN_UNRESOLVED_REFERENCES: &[&str] = &[
    "analytics_application_security_anomalies_report",
    "analytics_application_security_incidents_report",
    "analytics_application_security_network_report",
    "analytics_application_security_report",
    "analytics_asm_policy_changes_report",
    "analytics_bot_defense_event_report",
    "analytics_dns_cache_resolver_report",
    "analytics_dns_profile_report",
    "analytics_lsn_pool_report",
    "analytics_pool_traffic_report",
    "analytics_ssl_orchestrator_service_virtual_report",
    "analytics_system_monitor_report",
    "analytics_virtual_report",
    "apm_aaa_http_connector_transport",
    "apm_policy_access_policy",
    "apm_policy_agent_aaa_ocsp",
    "apm_policy_customization_group",
    "apm_policy_customization_languages",
    "apm_policy_image_file",
    "apm_policy_policy_item",
    "apm_policy_windows_group_policy_file",
    "apm_profile_remote_desktop",
    "asm_predefined_policy",
    "asm_response_code",
    "cm_cert",
    "gtm_monitor_none",
    "load",
    "ltm_classification_stats_application",
    "ltm_classification_stats_url_category",
    "ltm_monitor_none",
    "ltm_policy_strategy",
    "ltm_profile_ocsp_stapling_params",
    "net_interface_cos",
    "net_interface_ddm",
    "net_vlan_allowed",
    "restart",
    "save",
    "security_blacklist_publisher_all_blacklist_publisher",
    "security_blacklist_publisher_blacklist_publisher_stats",
    "security_blacklist_publisher_by_addr",
    "security_blacklist_publisher_by_category",
    "security_bot_defense_anomaly_category",
    "security_bot_defense_micro_service",
    "security_bot_defense_template",
    "security_dos_auto_thresholds_top_source_ips",
    "security_dos_virtual",
    "security_firewall_context_stat",
    "security_firewall_current_state",
    "security_firewall_ipi_category_info",
    "security_firewall_matching_rule",
    "security_firewall_rule_stat",
    "security_flowspec_route_injector_flowspec_advertised_route_info",
    "security_http_file_type",
    "security_packet_filter_rule_stat",
    "security_protocol_inspection_profile_status",
    "security_protocol_inspection_service",
    "security_protocol_inspection_virtual_servers",
    "security_scrubber_dwbl_scrubber_category_stats",
    "start",
    "stop",
    "sys_air_filter_reset",
    "sys_crypto_check_cert",
    "sys_crypto_crl",
    "sys_default_config",
    "sys_fpga_turboflex_profile",
    "sys_icall_event",
    "sys_icall_publisher",
    "sys_ipfix_destination",
    "sys_mcp_state",
    "sys_nethsm_async_queue_stat",
    "sys_nethsm_sync_queue_stat",
    "sys_sflow_data_source_http",
    "sys_sflow_data_source_interface",
    "sys_sflow_data_source_system",
    "sys_sflow_data_source_vlan",
    "sys_turboflex_profile_all",
    "sys_turboflex_profile_feature",
    "util_test_monitor",
    "wom_remote_route",
];

/// One bucket file's `kind` names, source-text order.
fn scan_bucket(path: &std::path::Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let kind_re = Regex::new(r#"kind:\s*"([^"]+)""#).expect("static regex");
    Ok(kind_re
        .captures_iter(&text)
        .map(|c| c[1].to_owned())
        .collect())
}

/// Every `references: &[...]` target across a bucket file's text, including
/// ones nested inside a `block: &[BigipPropertySpec { ... }]` — a plain text
/// scan sees those too, unlike a walk that only visits top-level specs.
fn scan_references(path: &std::path::Path) -> Result<BTreeSet<String>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let refs_re = Regex::new(r"references:\s*&\[([^\]]*)\]").expect("static regex");
    let str_re = Regex::new(r#""([^"]+)""#).expect("static regex");
    let mut out = BTreeSet::new();
    for caps in refs_re.captures_iter(&text) {
        for m in str_re.captures_iter(&caps[1]) {
            out.insert(m[1].to_owned());
        }
    }
    Ok(out)
}

struct Findings {
    /// `kind` name -> bucket files it appears in (>1 entry means duplicate).
    kind_locations: BTreeMap<String, Vec<&'static str>>,
    /// `(kind, wrong_bucket)` for a spec filed under a letter that doesn't
    /// match its first character.
    misfiled: Vec<(String, &'static str)>,
    /// Kinds the source-text scan found that the compiled registry didn't
    /// (or vice versa — see `registry_only`).
    scan_only: BTreeSet<String>,
    registry_only: BTreeSet<String>,
    /// Reference targets with no matching `kind`, minus the known-gap list.
    new_unresolved: BTreeSet<String>,
    /// `KNOWN_UNRESOLVED_REFERENCES` entries that resolve now (stale).
    stale_known_gaps: Vec<&'static str>,
}

fn analyse() -> Result<Findings> {
    let root = repo_root();
    let dir = root.join(DATA_DIR);

    let mut kind_locations: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    let mut misfiled = Vec::new();
    let mut scanned_kinds = BTreeSet::new();
    let mut all_references = BTreeSet::new();

    for &bucket in BUCKETS {
        let path = dir.join(format!("{bucket}.rs"));
        for kind in scan_bucket(&path)? {
            scanned_kinds.insert(kind.clone());
            kind_locations.entry(kind.clone()).or_default().push(bucket);
            let first = kind.chars().next().map(|c| c.to_ascii_lowercase());
            if first != bucket.chars().next() {
                misfiled.push((kind, bucket));
            }
        }
        all_references.extend(scan_references(&path)?);
    }

    let registry_kinds: BTreeSet<String> = tcl_registry::bigip::data::all_specs()
        .into_iter()
        .map(|spec| spec.kind_spec.kind.to_owned())
        .collect();

    let scan_only: BTreeSet<String> = scanned_kinds.difference(&registry_kinds).cloned().collect();
    let registry_only: BTreeSet<String> =
        registry_kinds.difference(&scanned_kinds).cloned().collect();

    let known_gaps: BTreeSet<&str> = KNOWN_UNRESOLVED_REFERENCES.iter().copied().collect();
    let new_unresolved: BTreeSet<String> = all_references
        .iter()
        .filter(|r| !scanned_kinds.contains(r.as_str()) && !known_gaps.contains(r.as_str()))
        .cloned()
        .collect();
    let stale_known_gaps: Vec<&'static str> = KNOWN_UNRESOLVED_REFERENCES
        .iter()
        .copied()
        .filter(|g| scanned_kinds.contains(*g))
        .collect();

    Ok(Findings {
        kind_locations,
        misfiled,
        scan_only,
        registry_only,
        new_unresolved,
        stale_known_gaps,
    })
}

fn is_clean(f: &Findings) -> bool {
    f.kind_locations.values().all(|locs| locs.len() == 1)
        && f.misfiled.is_empty()
        && f.scan_only.is_empty()
        && f.registry_only.is_empty()
        && f.new_unresolved.is_empty()
        && f.stale_known_gaps.is_empty()
}

/// Verify the BIG-IP object-spec data's internal consistency: unique,
/// correctly-filed `kind` names, scan/registry agreement, and reference
/// integrity against the documented known-gap list.
///
/// `check` is accepted for command-line symmetry with the other drift gates;
/// there is no generated form to write (see the module doc comment), so this
/// always verifies.
pub fn run(check: bool) -> Result<ExitCode> {
    let _ = check; // symmetry only — see the doc comment above
    let f = analyse()?;

    if is_clean(&f) {
        eprintln!(
            "OK: {DATA_DIR} is internally consistent ({} kinds, {} known reference gaps).",
            f.kind_locations.len(),
            KNOWN_UNRESOLVED_REFERENCES.len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    for (kind, locs) in f.kind_locations.iter().filter(|(_, v)| v.len() > 1) {
        eprintln!("duplicate kind {kind:?} appears in: {locs:?}");
    }
    for (kind, bucket) in &f.misfiled {
        eprintln!("{kind:?} is filed in {bucket}.rs but its first letter doesn't match");
    }
    if !f.scan_only.is_empty() {
        eprintln!(
            "kind(s) found by the source scan but not in the compiled registry \
             (scan/registry mismatch — check the `kind: \"...\"` pattern still matches): {:?}",
            f.scan_only
        );
    }
    if !f.registry_only.is_empty() {
        eprintln!(
            "kind(s) in the compiled registry but not found by the source scan \
             (a bucket file the scan didn't read — check BUCKETS matches data/mod.rs): {:?}",
            f.registry_only
        );
    }
    if !f.new_unresolved.is_empty() {
        eprintln!(
            "{} new unresolved reference target(s) not on the known-gap list \
             (fix the reference, or add it to KNOWN_UNRESOLVED_REFERENCES with a reason): {:?}",
            f.new_unresolved.len(),
            f.new_unresolved
        );
    }
    if !f.stale_known_gaps.is_empty() {
        eprintln!(
            "{} entr(y/ies) in KNOWN_UNRESOLVED_REFERENCES now resolve to a real kind \
             — remove them from the list: {:?}",
            f.stale_known_gaps.len(),
            f.stale_known_gaps
        );
    }
    Ok(ExitCode::from(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate this module exists to provide: the committed BIG-IP data
    /// files must be internally consistent right now. A failure here means
    /// the data changed in a way that broke uniqueness, filing, scan/registry
    /// agreement, or reference integrity — the same finding
    /// `cargo xtask bigip-data-schema --check` would report.
    #[test]
    fn committed_bigip_data_is_internally_consistent() {
        let f = analyse().expect("analyse committed BIG-IP data");
        let dups: Vec<_> = f
            .kind_locations
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .collect();
        assert!(dups.is_empty(), "duplicate kind names: {dups:?}");
        assert!(f.misfiled.is_empty(), "misfiled kinds: {:?}", f.misfiled);
        assert!(
            f.scan_only.is_empty(),
            "scan found kinds the registry doesn't have: {:?}",
            f.scan_only
        );
        assert!(
            f.registry_only.is_empty(),
            "registry has kinds the scan didn't find: {:?}",
            f.registry_only
        );
        assert!(
            f.new_unresolved.is_empty(),
            "new unresolved reference targets, not on the known-gap list: {:?}",
            f.new_unresolved
        );
        assert!(
            f.stale_known_gaps.is_empty(),
            "known-gap entries that now resolve — remove from the list: {:?}",
            f.stale_known_gaps
        );
    }

    #[test]
    fn scan_bucket_extracts_kind_names() {
        let dir =
            std::env::temp_dir().join(format!("bigip-data-schema-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("x.rs");
        std::fs::write(
            &path,
            "pub static SPECS: &[BigipObjectSpec] = &[\n\
             BigipObjectSpec { kind_spec: BigipObjectKindSpec { kind: \"x_one\", .. }, .. },\n\
             BigipObjectSpec { kind_spec: BigipObjectKindSpec { kind: \"x_two\", .. }, .. },\n\
             ];\n",
        )
        .expect("write fixture");
        let kinds = scan_bucket(&path).expect("scan fixture");
        assert_eq!(kinds, vec!["x_one".to_owned(), "x_two".to_owned()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_references_finds_nested_block_references() {
        let dir = std::env::temp_dir().join(format!(
            "bigip-data-schema-test-refs-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("x.rs");
        std::fs::write(
            &path,
            "references: &[\"top_level_target\"],\n\
             block: &[BigipPropertySpec { references: &[\"nested_target\"], .. }],\n",
        )
        .expect("write fixture");
        let refs = scan_references(&path).expect("scan fixture");
        assert!(refs.contains("top_level_target"));
        assert!(refs.contains("nested_target"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
