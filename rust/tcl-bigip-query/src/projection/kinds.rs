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

//! The module names and object kinds the query language documents, and their labels.

// Module / kind tables

/// The module names exposed at `<root>`.
pub(super) const MODULE_NAMES: &[&str] = &[
    "ltm",
    "net",
    "sys",
    "cm",
    "gtm",
    "apm",
    "security",
    "pem",
    "auth",
    "vcmp",
    "cli",
    "api-protection",
    "asm",
    "ilx",
    "wom",
    "analytics",
];

/// Every kind the projection covers, as `(tmsh_kind, label)`.
///
/// The TMSH kind is what [`placed_kind`] returns for a parsed object and what
/// a [`PathRef`] records as its expected target; the label is the identifier
/// its module container exposes it under (`.net["route-domain"]`). The label
/// is not derivable from the kind — `apm policy access-policy` is reached as
/// `access-policy`, not `policy-access-policy`, and `net tunnels tunnel` as
/// `tunnel` — so both halves are spelled out.
///
/// A kind absent from this table is not navigable: its module container
/// reports `no entry`, even when the parser types the stanza. The
/// `every_parsed_kind_is_projected_or_listed` test holds that deliberate,
/// pinning the parsed-but-unprojected kinds against the committed fixtures.
///
/// Ordering is by module, then the order each module's kinds are listed in
/// its container.
pub(super) const KINDS: &[(&str, &str)] = &[
    // ltm — the core load-balancing objects the cookbook and common queries
    // walk. The long-tail LTM kinds the Rust model carries no typed struct
    // for are simply absent.
    ("ltm virtual", "virtual"),
    ("ltm virtual-address", "virtual-address"),
    ("ltm pool", "pool"),
    ("ltm node", "node"),
    ("ltm rule", "rule"),
    ("ltm profile", "profile"),
    ("ltm monitor", "monitor"),
    ("ltm persistence", "persistence"),
    ("ltm snatpool", "snatpool"),
    ("ltm policy", "policy"),
    ("ltm data-group", "data-group"),
    ("ltm snat-translation", "snat-translation"),
    // net — the L2/L3 underlay the LTM tier sits on. `net self` / `net vlan`
    // / `net route-domain` are what a self-IP or VLAN-binding audit walks.
    ("net route", "route"),
    ("net vlan", "vlan"),
    ("net self", "self"),
    ("net route-domain", "route-domain"),
    ("net port-list", "port-list"),
    ("net interface", "interface"),
    ("net dns-resolver", "dns-resolver"),
    ("net tunnels tunnel", "tunnel"),
    ("net stp", "stp"),
    // sys — the filestore kinds carry the cert metadata `x509_from_config` /
    // `ucs_cert` project. `dns` / `ntp` / `snmp` / `global-settings` are TMSH
    // singletons: they parse with an empty full-path, so they hold exactly
    // one entry each and are read by streaming (`.sys.dns[]`).
    ("sys dns", "dns"),
    ("sys ntp", "ntp"),
    ("sys snmp", "snmp"),
    ("sys global-settings", "global-settings"),
    ("sys provision", "provision"),
    ("sys folder", "folder"),
    ("sys file ssl-cert", "file-ssl-cert"),
    ("sys file ssl-key", "file-ssl-key"),
    ("sys management-route", "management-route"),
    // cm — `device` + `device-group` are the HA topology an estate report
    // joins on; `cert` / `key` are the device-trust key material, carrying
    // the same cert metadata fields as `sys file ssl-cert`.
    ("cm cert", "cert"),
    ("cm key", "key"),
    ("cm device", "device"),
    ("cm device-group", "device-group"),
    ("cm traffic-group", "traffic-group"),
    ("cm trust-domain", "trust-domain"),
    ("cm ha-group", "ha-group"),
    // gtm — a GTM (DNS) tier fronts one or more LTM tiers: a `gtm server`'s
    // `virtual-servers` destinations are the downstream LTM virtual
    // addresses, which is how a report links a GTM to the LTMs it balances.
    ("gtm datacenter", "datacenter"),
    ("gtm server", "server"),
    ("gtm pool", "pool"),
    ("gtm wideip", "wideip"),
    ("gtm listener", "listener"),
    // apm — an access-policy's `start-item` / `items[]` deref into
    // `policy-item`, and an item's `agents[]` into `policy agent`, so a
    // policy walk resolves in one chain.
    ("apm policy access-policy", "access-policy"),
    ("apm policy policy-item", "policy-item"),
    ("apm policy agent", "policy-agent"),
    ("apm policy customization-source", "customization-source"),
    ("apm oauth db-instance", "oauth-db-instance"),
    (
        "apm ephemeral-auth ssh-security-config",
        "ssh-security-config",
    ),
    ("apm report default-report", "default-report"),
    // security — the AFM firewall + NAT posture alongside the LTM/GTM estate.
    ("security firewall policy", "firewall-policy"),
    ("security firewall rule-list", "firewall-rule-list"),
    ("security firewall address-list", "firewall-address-list"),
    ("security firewall port-list", "firewall-port-list"),
    ("security nat policy", "nat-policy"),
    ("security nat source-translation", "nat-source-translation"),
    (
        "security nat destination-translation",
        "nat-destination-translation",
    ),
];

/// The `(label, tmsh_kind)` pairs a module container exposes, in table order.
///
/// A kind belongs to the module its TMSH kind starts with, so the module list
/// follows from [`KINDS`] rather than a second table that can drift from it.
pub(super) fn module_kinds(
    module: &str,
) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
    KINDS.iter().filter_map(move |(kind, label)| {
        kind.strip_prefix(module)
            .and_then(|rest| rest.strip_prefix(' '))
            .map(|_| (*label, *kind))
    })
}

/// Every projected kind as `(tmsh_kind, label)`, for the docs gate that holds
/// `--help-dsl`'s MODULES section to this table.
#[cfg(test)]
pub(crate) fn documented_kind_labels() -> impl Iterator<Item = (&'static str, &'static str)> {
    KINDS.iter().copied()
}

/// Whether *kind* is a projected leaf object kind. Used by
/// `Container.is_object_kind`.
pub(super) fn is_object_kind_alias(kind: &str) -> bool {
    KINDS.iter().any(|(k, _)| *k == kind)
}

/// Map a kind to its label (for `PathRef` container navigation).
pub(super) fn kind_to_label(kind: &str) -> Option<&'static str> {
    KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, label)| *label)
}
