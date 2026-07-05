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

//! Typed BIG-IP scalar values — IP addresses, networks, ports,
//! partitions, destinations, folders, attachments, and the typed list.
//!
//! Each type round-trips to its canonical F5 spelling via `Display` and
//! parses via `parse` / `try_parse`, so reconstructed objects compare
//! equal.

mod ip_class;

pub mod address;
pub mod attachments;
pub mod bigip_list;
pub mod cert_key_chain;
pub mod data_group_record;
pub mod destination;
pub mod error;
pub mod firewall_rule;
pub mod folder;
pub mod gtm_region_member;
pub mod ip_range;
pub mod monitor_expression;
pub mod network;
pub mod partition;
pub mod policy;
pub mod port;
pub mod port_set;
pub mod route_domain;
pub mod snat_mode;

pub use address::{Address, FQDN, IPAddress, parse_address, try_parse_address};
pub use attachments::{PersistenceAttachment, ProfileAttachment};
pub use bigip_list::{BigipList, ListItem, ListItemValue, ListSyntax, SourceSpan};
pub use cert_key_chain::CertKeyChain;
pub use data_group_record::DataGroupRecord;
pub use destination::Destination;
pub use error::ValueError;
pub use firewall_rule::{FirewallEndpoint, FirewallRule, NatRule};
pub use folder::{Folder, ObjectPath};
pub use gtm_region_member::GtmRegionMember;
pub use ip_range::IPRange;
pub use monitor_expression::{MonitorExpression, MonitorMode};
pub use network::{Cidr, Network};
pub use partition::Partition;
pub use policy::{LtmPolicyAction, LtmPolicyCondition};
pub use port::{Port, PortRange};
pub use port_set::{PortSegment, PortSet};
pub use route_domain::RouteDomain;
pub use snat_mode::{SnatMode, SnatModeKind};
