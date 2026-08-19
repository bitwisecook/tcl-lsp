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
//! `rust/tcl-registry/src/bigip/data/{analytics,apm,ltm,...}.rs` (798 object
//! kinds, one file per tmsh module word — `analytics`, `apm`, `ltm`, `gtm`,
//! …, derived from each spec's own `module: Some("...")` field, which every
//! one of the 798 specs already carries) carried a `Generated ... DO NOT
//! EDIT` header naming a `scripts/registry-audit/gen_bigip_rust.py`
//! generator that no longer exists in this branch's tree, and its own
//! history — the commits that actually ran it against the pre-rewrite
//! Python registry (`dialects/f5/bigip/registry/specs/`, still present on
//! `main`) — was intentionally squashed away by the `rust` branch's
//! rebase-onto-main commit. There is nothing left in this branch's git
//! history to replay, so the header now says what these files actually are:
//! hand-maintained. (Originally organised by the first letter of `kind`;
//! reorganised by tmsh module name per maintainer review on the PR that
//! introduced this gate — a module word groups related objects far more
//! usefully than an arbitrary initial letter does.)
//!
//! What a generator would otherwise have guaranteed for free is instead
//! enforced here as a structural drift gate:
//!
//! - **module-list agreement, from three independent sources** — the `.rs`
//!   files physically present in `data/` (the filesystem), the `mod x;`
//!   declarations in `data/mod.rs` (what the crate actually compiles), and
//!   the module names the source-text scan (below) finds. No source is a
//!   hand-kept constant this xtask maintains itself: a module file added to
//!   the directory but never wired into `data/mod.rs` at all — not even
//!   half-wired, the shape a `mod x;`-vs-`BUCKETS` diff alone cannot see
//!   either, since an omission from *both* sides of a two-way comparison
//!   agrees with itself — is still caught, because the directory listing
//!   doesn't depend on `data/mod.rs` knowing about the file in the first
//!   place;
//! - **uniqueness** — no `kind` name appears in more than one spec;
//! - **filing** — every spec lives in the module file matching its own
//!   `module` field (a spec pasted into the wrong module file, or a
//!   `module` field changed without moving the spec, both show up here);
//! - **scan/registry agreement** — the source-text scan this module does
//!   (regex over `kind: "..."` — the per-module data modules are private to
//!   `tcl-registry`, so an xtask outside that crate cannot enumerate them
//!   per-file through the public API) must find exactly the same kind set
//!   the compiled registry ([`tcl_registry::bigip::data::all_specs`])
//!   reports — catching a module declared in `data/mod.rs` (`mod x;`) but
//!   never aggregated into `all_specs()`'s `v.extend(...)` chain (the
//!   "half-wired" case);
//! - **reference integrity** — every `BigipPropertySpec::references` entry
//!   either names a real `kind`, or is on [`KNOWN_UNRESOLVED_REFERENCES`],
//!   the documented pre-existing gap list discovered when this gate was
//!   introduced (tmsh action verbs like `start`/`stop`/`save` captured
//!   alongside real object-kind references, and object kinds the dataset
//!   never carried a spec for). A *new* unresolved reference — not on that
//!   list — fails the gate; a list entry that is no longer a gap (the name
//!   now resolves, or nothing references it anymore) also fails the gate,
//!   so the list cannot silently drift stale in either direction.

use std::collections::BTreeSet;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

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

/// Every `.rs` file physically present in `data/`, minus `mod.rs` itself —
/// the filesystem's own answer to "what module files exist", independent of
/// whether anything in `data/mod.rs` actually references them. This is the
/// only one of the three module-list sources that can catch a file that was
/// added to the directory but never wired into `mod.rs` in any way (not
/// even a bare `mod x;` with no `extend` call) — a `mod x;`-vs-hand-kept-
/// constant diff cannot see that shape, because both sides omit the new
/// file identically and agree with each other.
fn scan_directory_modules(dir: &std::path::Path) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for entry in
        fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("reading directory {}", dir.display()))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && stem != "mod"
        {
            out.insert(stem.to_owned());
        }
    }
    Ok(out)
}

/// The `mod x;` declarations in `data/mod.rs`, source-text order.
fn scan_declared_modules(mod_rs: &std::path::Path) -> Result<BTreeSet<String>> {
    let text =
        fs::read_to_string(mod_rs).with_context(|| format!("reading {}", mod_rs.display()))?;
    let decl_re = Regex::new(r"^mod ([a-z_]+);").expect("static regex");
    Ok(text
        .lines()
        .filter_map(|line| decl_re.captures(line))
        .map(|c| c[1].to_owned())
        .collect())
}

/// One module file's `(kind, module_field)` pairs, source-text order. The
/// `module_field` is the spec's own `module: Some("...")` value (tmsh's
/// original word, which may contain a hyphen — `api-protection` — where the
/// file/identifier form uses an underscore instead, since Rust module names
/// cannot contain hyphens); see [`module_ident`].
fn scan_module_file(path: &std::path::Path) -> Result<Vec<(String, String)>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let kind_spec_re =
        Regex::new(r"kind_spec:\s*BigipObjectKindSpec\s*\{([^}]*)\}").expect("static regex");
    let kind_re = Regex::new(r#"kind:\s*"([^"]+)""#).expect("static regex");
    let module_re = Regex::new(r#"module:\s*Some\("([^"]+)"\)"#).expect("static regex");
    let mut out = Vec::new();
    for caps in kind_spec_re.captures_iter(&text) {
        let block = &caps[1];
        let Some(kind) = kind_re.captures(block).map(|c| c[1].to_owned()) else {
            continue;
        };
        let Some(module) = module_re.captures(block).map(|c| c[1].to_owned()) else {
            continue;
        };
        out.push((kind, module));
    }
    Ok(out)
}

/// Every `references: &[...]` target across a module file's text, including
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

/// tmsh module word -> the Rust module/file identifier it is filed under
/// (hyphens, the only character tmsh module words use that Rust identifiers
/// cannot, become underscores — `api-protection` files under
/// `api_protection.rs`).
fn module_ident(module_field: &str) -> String {
    module_field.replace('-', "_")
}

/// Which entries of a known-reference-gap allow-list are no longer a gap.
///
/// An entry is stale in either of two independent ways: the name now
/// resolves to a real `kind` (the gap was fixed by adding the missing
/// spec), *or* nothing references it anymore (the gap was fixed by
/// removing or correcting the reference itself, not the kind). The second
/// case is not implied by the first: a reference that has been deleted
/// trivially still fails `scanned_kinds.contains`, so checking only "does
/// it resolve now" leaves a gap that was fixed by removing the reference
/// classified as an active gap forever.
fn find_stale_known_gaps(
    known_gaps: &[&'static str],
    scanned_kinds: &BTreeSet<String>,
    all_references: &BTreeSet<String>,
) -> Vec<&'static str> {
    known_gaps
        .iter()
        .copied()
        .filter(|g| scanned_kinds.contains(*g) || !all_references.contains(*g))
        .collect()
}

struct Findings {
    /// `.rs` files in `data/` not declared via `mod x;` in `data/mod.rs` —
    /// a module file that was never wired in at all.
    files_not_declared: Vec<String>,
    /// `mod x;` declarations in `data/mod.rs` with no matching `.rs` file.
    declared_files_missing: Vec<String>,
    /// `kind` name -> module files it appears in (>1 entry means duplicate).
    kind_locations: std::collections::BTreeMap<String, Vec<String>>,
    /// `(kind, declared_module_field, filed_under)` for a spec whose own
    /// `module` field doesn't match the file it's filed under.
    misfiled: Vec<(String, String, String)>,
    /// Kinds the source-text scan found that the compiled registry didn't
    /// (or vice versa — see `registry_only`).
    scan_only: BTreeSet<String>,
    registry_only: BTreeSet<String>,
    /// Reference targets with no matching `kind`, minus the known-gap list.
    new_unresolved: BTreeSet<String>,
    /// `KNOWN_UNRESOLVED_REFERENCES` entries that are no longer a gap —
    /// either because the name now resolves to a real `kind`, or because
    /// nothing references it anymore (the reference itself was removed or
    /// fixed, not just the missing kind added).
    stale_known_gaps: Vec<&'static str>,
}

fn analyse() -> Result<Findings> {
    let root = repo_root();
    let dir = root.join(DATA_DIR);

    let directory_modules = scan_directory_modules(&dir)?;
    let declared_modules = scan_declared_modules(&dir.join("mod.rs"))?;
    let files_not_declared: Vec<String> = directory_modules
        .difference(&declared_modules)
        .cloned()
        .collect();
    let declared_files_missing: Vec<String> = declared_modules
        .difference(&directory_modules)
        .cloned()
        .collect();

    let mut kind_locations: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut misfiled = Vec::new();
    let mut scanned_kinds = BTreeSet::new();
    let mut all_references = BTreeSet::new();

    // Scan every module actually declared (the modules the crate compiles);
    // a file present but not declared is already reported above and would
    // not compile into the crate anyway, so it is not double-scanned here.
    for module in &declared_modules {
        let path = dir.join(format!("{module}.rs"));
        if !path.is_file() {
            continue; // reported as declared_files_missing above
        }
        for (kind, module_field) in scan_module_file(&path)? {
            scanned_kinds.insert(kind.clone());
            kind_locations
                .entry(kind.clone())
                .or_default()
                .push(module.clone());
            if module_ident(&module_field) != *module {
                misfiled.push((kind, module_field, module.clone()));
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
    let stale_known_gaps =
        find_stale_known_gaps(KNOWN_UNRESOLVED_REFERENCES, &scanned_kinds, &all_references);

    Ok(Findings {
        files_not_declared,
        declared_files_missing,
        kind_locations,
        misfiled,
        scan_only,
        registry_only,
        new_unresolved,
        stale_known_gaps,
    })
}

fn is_clean(f: &Findings) -> bool {
    f.files_not_declared.is_empty()
        && f.declared_files_missing.is_empty()
        && f.kind_locations.values().all(|locs| locs.len() == 1)
        && f.misfiled.is_empty()
        && f.scan_only.is_empty()
        && f.registry_only.is_empty()
        && f.new_unresolved.is_empty()
        && f.stale_known_gaps.is_empty()
}

/// Verify the BIG-IP object-spec data's internal consistency: a module-list
/// that agrees across the filesystem/`mod.rs`/registry, unique correctly-
/// filed `kind` names, and reference integrity against the documented
/// known-gap list.
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

    if !f.files_not_declared.is_empty() {
        eprintln!(
            "{} module file(s) exist in {DATA_DIR} but are never declared in {DATA_DIR}/mod.rs \
             (add `mod x;` and `v.extend(x::SPECS.iter());`): {:?}",
            f.files_not_declared.len(),
            f.files_not_declared
        );
    }
    if !f.declared_files_missing.is_empty() {
        eprintln!(
            "{} `mod x;` declaration(s) in {DATA_DIR}/mod.rs have no matching .rs file: {:?}",
            f.declared_files_missing.len(),
            f.declared_files_missing
        );
    }
    for (kind, locs) in f.kind_locations.iter().filter(|(_, v)| v.len() > 1) {
        eprintln!("duplicate kind {kind:?} appears in: {locs:?}");
    }
    for (kind, declared_module, filed_under) in &f.misfiled {
        eprintln!("{kind:?} declares module {declared_module:?} but is filed in {filed_under}.rs");
    }
    if !f.scan_only.is_empty() {
        eprintln!(
            "kind(s) found by the source scan but not in the compiled registry \
             (a module declared in mod.rs but never added to all_specs()'s extend chain): {:?}",
            f.scan_only
        );
    }
    if !f.registry_only.is_empty() {
        eprintln!(
            "kind(s) in the compiled registry but not found by the source scan \
             (check mod.rs's `mod x;` declarations match its extend chain): {:?}",
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
            "{} entr(y/ies) in KNOWN_UNRESOLVED_REFERENCES are no longer a gap — either the \
             name now resolves to a real kind, or nothing references it anymore — \
             remove them from the list: {:?}",
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
    /// the data changed in a way that broke module-list agreement,
    /// uniqueness, filing, scan/registry agreement, or reference integrity —
    /// the same finding `cargo xtask bigip-data-schema --check` would report.
    #[test]
    fn committed_bigip_data_is_internally_consistent() {
        let f = analyse().expect("analyse committed BIG-IP data");
        assert!(
            f.files_not_declared.is_empty(),
            "module file(s) not declared in mod.rs: {:?}",
            f.files_not_declared
        );
        assert!(
            f.declared_files_missing.is_empty(),
            "mod.rs declares module(s) with no file: {:?}",
            f.declared_files_missing
        );
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
            "known-gap entries that are no longer a gap — remove from the list: {:?}",
            f.stale_known_gaps
        );
    }

    /// A known-gap entry that now resolves to a real `kind` is stale (the
    /// original, already-covered direction).
    #[test]
    fn stale_known_gaps_flags_an_entry_that_now_resolves() {
        let scanned: BTreeSet<String> = ["now_a_real_kind".to_owned()].into_iter().collect();
        let refs: BTreeSet<String> = ["now_a_real_kind".to_owned()].into_iter().collect();
        let stale = find_stale_known_gaps(&["now_a_real_kind"], &scanned, &refs);
        assert_eq!(stale, vec!["now_a_real_kind"]);
    }

    /// Regression test for the exact gap an adversarial review found: a
    /// known-gap entry whose *reference was removed entirely* — nobody
    /// names it anymore, so there is nothing left to resolve — must also be
    /// flagged stale. The original implementation only checked "does it
    /// resolve now", so a gap fixed by deleting the bad reference (rather
    /// than by adding the missing kind) stayed on the list forever.
    #[test]
    fn stale_known_gaps_flags_an_entry_whose_reference_was_removed() {
        let scanned: BTreeSet<String> = BTreeSet::new();
        let refs: BTreeSet<String> = BTreeSet::new(); // nobody references it anymore
        let stale = find_stale_known_gaps(&["nobody_references_this_anymore"], &scanned, &refs);
        assert_eq!(stale, vec!["nobody_references_this_anymore"]);
    }

    /// A genuine, still-open gap — referenced, unresolved — is not flagged.
    #[test]
    fn stale_known_gaps_does_not_flag_a_genuine_open_gap() {
        let scanned: BTreeSet<String> = BTreeSet::new();
        let refs: BTreeSet<String> = ["still_a_real_gap".to_owned()].into_iter().collect();
        let stale = find_stale_known_gaps(&["still_a_real_gap"], &scanned, &refs);
        assert!(stale.is_empty());
    }

    #[test]
    fn module_ident_converts_hyphens_to_underscores() {
        assert_eq!(module_ident("api-protection"), "api_protection");
        assert_eq!(module_ident("ltm"), "ltm");
    }

    /// Regression test for the exact gap an adversarial review found: a
    /// module file physically added to `data/` but never declared via
    /// `mod x;` in `data/mod.rs` at all (not even half-wired — no `mod x;`
    /// and no `extend` call) is invisible to a `mod x;`-vs-hand-kept-
    /// constant comparison, because a constant that also never learns about
    /// the new file agrees with `mod.rs`'s omission. Reading the directory
    /// itself as an independent third source of truth catches it.
    #[test]
    fn directory_vs_declared_flags_a_file_never_wired_into_mod_rs() {
        let dir = std::env::temp_dir().join(format!(
            "bigip-data-schema-test-orphan-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("mod.rs"), "mod ltm;\nmod apm;\n").expect("write mod.rs");
        std::fs::write(dir.join("ltm.rs"), "// ltm\n").expect("write ltm.rs");
        std::fs::write(dir.join("apm.rs"), "// apm\n").expect("write apm.rs");
        // `orphan.rs` exists on disk but mod.rs never mentions it.
        std::fs::write(dir.join("orphan.rs"), "// never wired in\n").expect("write orphan.rs");

        let directory = scan_directory_modules(&dir).expect("scan directory");
        let declared = scan_declared_modules(&dir.join("mod.rs")).expect("scan mod.rs");
        let not_declared: Vec<&String> = directory.difference(&declared).collect();
        assert_eq!(not_declared, vec![&"orphan".to_owned()]);
        let missing_files: Vec<&String> = declared.difference(&directory).collect();
        assert!(missing_files.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The reverse omission: `mod.rs` still declares a module whose file
    /// was deleted (or renamed) without updating `mod.rs`.
    #[test]
    fn directory_vs_declared_flags_a_declared_module_with_no_file() {
        let dir = std::env::temp_dir().join(format!(
            "bigip-data-schema-test-missingfile-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("mod.rs"), "mod ltm;\nmod gone;\n").expect("write mod.rs");
        std::fs::write(dir.join("ltm.rs"), "// ltm\n").expect("write ltm.rs");

        let directory = scan_directory_modules(&dir).expect("scan directory");
        let declared = scan_declared_modules(&dir.join("mod.rs")).expect("scan mod.rs");
        let missing_files: Vec<&String> = declared.difference(&directory).collect();
        assert_eq!(missing_files, vec![&"gone".to_owned()]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_module_file_extracts_kind_and_module() {
        let dir = std::env::temp_dir().join(format!(
            "bigip-data-schema-test-scanmod-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("ltm.rs");
        std::fs::write(
            &path,
            "pub static SPECS: &[BigipObjectSpec] = &[\n\
             BigipObjectSpec { kind_spec: BigipObjectKindSpec { kind: \"ltm_pool\", module: Some(\"ltm\"), .. }, .. },\n\
             ];\n",
        )
        .expect("write fixture");
        let pairs = scan_module_file(&path).expect("scan fixture");
        assert_eq!(pairs, vec![("ltm_pool".to_owned(), "ltm".to_owned())]);
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
