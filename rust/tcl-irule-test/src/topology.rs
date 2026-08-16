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

//! SCF topology bridge — generate orchestrator setup Tcl from a BIG-IP config.
//!
//! Parses a bigip.conf / SCF source via [`tcl_bigip`] and emits the `::orch::`
//! commands that configure the iRule-test orchestrator for a given virtual
//! server: profiles, the VIP address/port, pools + members, data-groups, and
//! the attached iRules.

use std::fmt::Write as _;

use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::{BigipConfig, parse_bigip_conf};
use tcl_bigip::value::ListItemValue;
use tcl_syntax::list::{join_list, list_element};

/// Errors raised when generating a topology setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// No virtual server matched the requested name.
    VirtualServerNotFound(String),
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VirtualServerNotFound(name) => {
                write!(f, "Virtual server {name:?} not found")
            }
        }
    }
}

impl std::error::Error for TopologyError {}

/// A parsed BIG-IP config that can emit orchestrator setup for its virtual
/// servers.
pub struct Topology {
    config: BigipConfig,
}

impl Topology {
    /// Build from an already-parsed config.
    #[must_use]
    pub fn new(config: BigipConfig) -> Self {
        Self { config }
    }

    /// Parse a bigip.conf / SCF source string (partition defaults to `Common`).
    #[must_use]
    pub fn from_source(source: &str) -> Self {
        Self::new(parse_bigip_conf(source, "Common"))
    }

    /// The parsed config.
    #[must_use]
    pub fn config(&self) -> &BigipConfig {
        &self.config
    }

    /// Full paths of every virtual server, in source order.
    #[must_use]
    pub fn virtual_servers(&self) -> Vec<&str> {
        self.config
            .objects
            .iter()
            .filter(|p| p.table_name == "virtual_servers")
            .map(|p| p.full_path.as_str())
            .collect()
    }

    /// Generate the `::orch::` setup Tcl for `vs_name`.
    ///
    /// Mirrors `TopologyFromSCF.generate_tcl_setup`: configure profiles + VIP,
    /// register every pool (by full path and short name) with members, register
    /// data-groups, and load the VS's attached iRules.
    ///
    /// # Errors
    /// [`TopologyError::VirtualServerNotFound`] when no VS matches `vs_name`.
    #[allow(clippy::too_many_lines)] // emits the setup sections in their script order
    pub fn generate_tcl_setup(&self, vs_name: &str) -> Result<String, TopologyError> {
        let vs = self
            .resolve_virtual(vs_name)
            .ok_or_else(|| TopologyError::VirtualServerNotFound(vs_name.to_owned()))?;
        let vs_path = vs.name.as_str();

        let mut out = String::new();
        let _ = writeln!(out, "# Auto-generated from SCF topology");
        let _ = writeln!(out, "# Virtual server: {vs_path}");
        out.push('\n');

        // Profiles. Ensure a transport profile is present if any are.
        let mut profile_types = self.resolve_profile_types(vs);
        if !profile_types.is_empty() && !profile_types.iter().any(|t| t == "TCP" || t == "UDP") {
            profile_types.insert(0, "TCP".to_owned());
        }
        let profiles_tcl = if profile_types.is_empty() {
            "TCP HTTP".to_owned()
        } else {
            profile_types.join(" ")
        };
        let _ = writeln!(out, "::orch::configure -profiles {{{profiles_tcl}}}");

        // VIP address / port.
        if let Some(dest) = &vs.destination {
            let _ = writeln!(
                out,
                "::orch::configure -local_addr {} -local_port {}",
                dest.address, dest.port
            );
        }
        out.push('\n');

        // Pools (full path + short name) with members.
        let mut any_pool = false;
        for placed in &self.config.objects {
            if placed.table_name != "pools" {
                continue;
            }
            let ModelObject::Pool(pool) = &placed.object else {
                continue;
            };
            let members = pool_member_names(pool);
            if members.is_empty() {
                continue;
            }
            any_pool = true;
            let members_tcl = join_list(&members);
            let key = placed.full_path.as_str();
            let _ = writeln!(
                out,
                "::orch::add_pool {} {}",
                list_element(key),
                list_element(&members_tcl)
            );
            let short = short_name(key);
            if short != key {
                let _ = writeln!(
                    out,
                    "::orch::add_pool {} {}",
                    list_element(short),
                    list_element(&members_tcl)
                );
            }
        }
        if any_pool {
            out.push('\n');
        }

        // Data-groups (full path + short name).
        let mut any_dg = false;
        for placed in &self.config.objects {
            if placed.table_name != "data_groups" {
                continue;
            }
            let ModelObject::DataGroup(dg) = &placed.object else {
                continue;
            };
            any_dg = true;
            let dg_type = if dg.value_type.is_empty() {
                "string"
            } else {
                dg.value_type.as_str()
            };
            let records_tcl = datagroup_records_tcl(&dg.records);
            let key = placed.full_path.as_str();
            let _ = writeln!(
                out,
                "::orch::add_datagroup {} {} {}",
                list_element(key),
                list_element(dg_type),
                list_element(&records_tcl)
            );
            let short = short_name(key);
            if short != key {
                let _ = writeln!(
                    out,
                    "::orch::add_datagroup {} {} {}",
                    list_element(short),
                    list_element(dg_type),
                    list_element(&records_tcl)
                );
            }
        }
        if any_dg {
            out.push('\n');
        }

        // Attached iRules.
        for rule_ref in vs_rule_refs(vs) {
            if let Some(source) = self.rule_source(&rule_ref) {
                let _ = writeln!(out, "::orch::load_irule {}", list_element(source));
            }
        }
        out.push('\n');

        Ok(out)
    }

    /// Generate a complete standalone test script for `vs_name`.
    ///
    /// # Errors
    /// [`TopologyError::VirtualServerNotFound`] when no VS matches `vs_name`.
    pub fn generate_full_test_script(&self, vs_name: &str) -> Result<String, TopologyError> {
        let setup = self.generate_tcl_setup(vs_name)?;
        let mut out = String::new();
        let _ = writeln!(out, "#!/usr/bin/env tclsh");
        let _ = writeln!(out, "#");
        let _ = writeln!(out, "# Test script for virtual server: {vs_name}");
        let _ = writeln!(
            out,
            "# Auto-generated -- customise the test scenarios below."
        );
        let _ = writeln!(out, "#\n");
        let _ = writeln!(out, "set script_dir [file dirname [info script]]");
        let _ = writeln!(out, "source [file join $script_dir orchestrator.tcl]");
        let _ = writeln!(out, "source [file join $script_dir scf_loader.tcl]\n");
        let _ = writeln!(out, "::orch::init\n");
        let _ = writeln!(out, "# Topology setup\n");
        out.push_str(&setup);
        let _ = writeln!(out, "\n# Test scenarios\n");
        let _ = writeln!(
            out,
            "::orch::run_http_request -method GET -uri \"/\" -host \"example.com\"\n"
        );
        let _ = writeln!(out, "::orch::summary");
        Ok(out)
    }

    /// Resolve a virtual server by full path, short name, or `/Common/`-prefixed
    /// name. Mirrors `resolve_name` over the `virtual_servers` table.
    fn resolve_virtual(&self, name: &str) -> Option<&tcl_bigip::model::BigipVirtualServer> {
        let mut by_short: Option<&tcl_bigip::model::BigipVirtualServer> = None;
        for placed in &self.config.objects {
            if placed.table_name != "virtual_servers" {
                continue;
            }
            let ModelObject::VirtualServer(vs) = &placed.object else {
                continue;
            };
            if placed.full_path == name {
                return Some(vs);
            }
            if short_name(&placed.full_path) == name
                || placed.full_path == format!("/Common/{name}")
            {
                by_short.get_or_insert(vs);
            }
        }
        by_short
    }

    /// Resolve the TMM profile-type tags for a VS, mirroring
    /// `_resolve_profile_types` (inference path).
    ///
    /// Currently name-inference only; the refinement is to first resolve each
    /// profile *object* (`self.config`) and read its `ProfileType`, falling back
    /// to inference — which is why this stays a method on `self`.
    #[allow(clippy::unused_self)]
    fn resolve_profile_types(&self, vs: &tcl_bigip::model::BigipVirtualServer) -> Vec<String> {
        let mut types: Vec<String> = Vec::new();
        for pref in vs_profile_refs(vs) {
            if let Some(t) = infer_profile_type(&pref)
                && !types.iter().any(|existing| *existing == t)
            {
                types.push(t.to_owned());
            }
        }
        types
    }

    /// The source of an attached iRule, resolved by full path or short name.
    fn rule_source(&self, rule_ref: &str) -> Option<&str> {
        let mut by_short: Option<&str> = None;
        for placed in &self.config.objects {
            if placed.table_name != "rules" {
                continue;
            }
            let ModelObject::Rule(rule) = &placed.object else {
                continue;
            };
            if placed.full_path == rule_ref {
                return Some(rule.source.as_str());
            }
            if short_name(&placed.full_path) == rule_ref
                || placed.full_path == format!("/Common/{rule_ref}")
            {
                by_short.get_or_insert(rule.source.as_str());
            }
        }
        by_short
    }
}

/// The last `/`-separated component of a path (the short name).
fn short_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Member names of a pool (the `node:port` strings the orchestrator expects).
fn pool_member_names(pool: &tcl_bigip::model::BigipPool) -> Vec<String> {
    pool.members
        .items
        .iter()
        .filter_map(|item| match &item.value {
            ListItemValue::PoolMember(m) => Some(m.name.clone()),
            _ => None,
        })
        .collect()
}

/// iRule references attached to a VS, as their path strings.
fn vs_rule_refs(vs: &tcl_bigip::model::BigipVirtualServer) -> Vec<String> {
    vs.rules.paths()
}

/// Profile references on a VS, as their path strings.
fn vs_profile_refs(vs: &tcl_bigip::model::BigipVirtualServer) -> Vec<String> {
    vs.profiles.paths()
}

/// Render data-group records as the even-length `key {}` Tcl list the
/// orchestrator's `add_datagroup` expects.
fn datagroup_records_tcl(records: &[String]) -> String {
    join_list(records.iter().flat_map(|record| [record.as_str(), ""]))
}

/// Infer a TMM profile-type tag from a profile reference name.
fn infer_profile_type(pref: &str) -> Option<&'static str> {
    // Match by substring (not exact name) so a custom profile name like
    // `my_http_profile` / `web-tcp-opt` still maps to its base type — an exact
    // `name == "http"` test never matched a customised name, so the generated
    // orchestrator omitted the profile and `when HTTP_REQUEST` handlers never
    // fired (issue 190). More specific types are checked first (`fasthttp` and
    // the SSL variants before the plain `http`/`tcp` substrings).
    let name = short_name(pref).to_ascii_lowercase();
    if name.contains("clientssl") || name.contains("client-ssl") {
        Some("CLIENTSSL")
    } else if name.contains("serverssl") || name.contains("server-ssl") {
        Some("SERVERSSL")
    } else if name.contains("fasthttp") {
        Some("FASTHTTP")
    } else if name.contains("dns") {
        Some("DNS")
    } else if name.contains("http") {
        Some("HTTP")
    } else if name.contains("tcp") {
        Some("TCP")
    } else if name.contains("udp") {
        Some("UDP")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &str = "\
ltm pool web_pool {
    members {
        10.0.1.1:80 { address 10.0.1.1 }
        10.0.1.2:80 { address 10.0.1.2 }
    }
}
ltm rule redirect_rule {
when HTTP_REQUEST {
    if { [HTTP::host] eq \"api.example.com\" } {
        pool web_pool
    }
}
}
ltm virtual www_vs {
    destination 10.0.0.1:443
    profiles {
        clientssl { }
        http { }
    }
    rules {
        redirect_rule
    }
}
";

    #[test]
    fn lists_virtual_servers() {
        let topo = Topology::from_source(CONF);
        assert_eq!(topo.virtual_servers(), vec!["/Common/www_vs"]);
    }

    #[test]
    fn generates_setup_for_vs() {
        let topo = Topology::from_source(CONF);
        let setup = topo.generate_tcl_setup("www_vs").expect("setup");
        // Profiles inferred + TCP prepended; CLIENTSSL/HTTP from the names.
        assert!(
            setup.contains("::orch::configure -profiles {TCP CLIENTSSL HTTP}"),
            "{setup}"
        );
        // VIP from the destination.
        assert!(
            setup.contains("::orch::configure -local_addr 10.0.0.1 -local_port 443"),
            "{setup}"
        );
        // Pool with members, by full path and short name.
        assert!(
            setup.contains("::orch::add_pool /Common/web_pool {10.0.1.1:80 10.0.1.2:80}"),
            "{setup}"
        );
        assert!(
            setup.contains("::orch::add_pool web_pool {10.0.1.1:80 10.0.1.2:80}"),
            "{setup}"
        );
        // Attached iRule body loaded.
        assert!(setup.contains("::orch::load_irule {"), "{setup}");
        assert!(setup.contains("HTTP::host"), "{setup}");
    }

    #[test]
    fn unknown_vs_errors() {
        let topo = Topology::from_source(CONF);
        assert_eq!(
            topo.generate_tcl_setup("nope"),
            Err(TopologyError::VirtualServerNotFound("nope".to_owned()))
        );
    }

    #[test]
    fn full_test_script_wraps_setup() {
        let topo = Topology::from_source(CONF);
        let script = topo.generate_full_test_script("www_vs").expect("script");
        assert!(script.contains("source [file join $script_dir orchestrator.tcl]"));
        assert!(script.contains("::orch::init"));
        assert!(script.contains("::orch::add_pool /Common/web_pool"));
        assert!(script.contains("::orch::summary"));
    }

    #[test]
    fn infers_profile_types() {
        assert_eq!(infer_profile_type("/Common/clientssl"), Some("CLIENTSSL"));
        assert_eq!(infer_profile_type("http"), Some("HTTP"));
        assert_eq!(infer_profile_type("/Common/my-fasthttp"), Some("FASTHTTP"));
        assert_eq!(infer_profile_type("/Common/weird"), None);
    }

    #[test]
    fn datagroup_records_keep_arbitrary_keys_as_literal_list_elements() {
        let records = [
            "$variable".to_owned(),
            "[command]".to_owned(),
            "a \"quote\"".to_owned(),
            r"path\\with\\slashes".to_owned(),
            "unbalanced { brace".to_owned(),
        ];
        let rendered = datagroup_records_tcl(&records);
        let expected = records
            .iter()
            .flat_map(|record| [record.as_str(), ""])
            .collect::<Vec<_>>();
        assert_eq!(
            tcl_syntax::list::split_list(&rendered).unwrap(),
            expected,
            "records must remain literal list values"
        );
    }

    #[test]
    fn infers_profile_types_for_custom_names() {
        // Custom (non-canonical) profile names must still map to their base
        // type by substring, so the generated orchestrator wires the profile
        // and its events fire (issue 190).
        assert_eq!(infer_profile_type("/Common/my_http_profile"), Some("HTTP"));
        assert_eq!(infer_profile_type("/Common/web-tcp-opt"), Some("TCP"));
        assert_eq!(infer_profile_type("/Common/udp_datagram"), Some("UDP"));
        // Specific types still win over the plainer substrings.
        assert_eq!(
            infer_profile_type("/Common/site_fasthttp"),
            Some("FASTHTTP")
        );
        assert_eq!(
            infer_profile_type("/Common/custom-clientssl-1"),
            Some("CLIENTSSL")
        );
    }
}
