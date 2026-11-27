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

//! BIG-IP object specs for `security firewall`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod address_list;
mod config_change_log;
mod global_fqdn_policy;
mod global_rules;
mod management_ip_rules;
mod on_demand_compilation;
mod on_demand_rule_deploy;
mod policy;
mod port_list;
mod port_misuse_policy;
mod rule_list;
mod schedule;
mod user_domain;
mod user_list;
mod uuid_default_autogenerate;

/// The spec tables of `security firewall`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        address_list::SPECS,
        config_change_log::SPECS,
        global_fqdn_policy::SPECS,
        global_rules::SPECS,
        management_ip_rules::SPECS,
        on_demand_compilation::SPECS,
        on_demand_rule_deploy::SPECS,
        policy::SPECS,
        port_list::SPECS,
        port_misuse_policy::SPECS,
        rule_list::SPECS,
        schedule::SPECS,
        user_domain::SPECS,
        user_list::SPECS,
        uuid_default_autogenerate::SPECS,
    ]
}
