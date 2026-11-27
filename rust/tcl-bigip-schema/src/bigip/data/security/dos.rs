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

//! BIG-IP object specs for `security dos`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod autodos_file_object;
mod behavioral_signature;
mod bot_signature;
mod bot_signature_category;
mod device_config;
mod dos_signature;
mod dynamic_signatures;
mod ip_uncommon_protolist;
mod l4bdos_file_object;
mod network_whitelist;
mod profile;
mod udp_portlist;

/// The spec tables of `security dos`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        autodos_file_object::SPECS,
        behavioral_signature::SPECS,
        bot_signature::SPECS,
        bot_signature_category::SPECS,
        device_config::SPECS,
        dos_signature::SPECS,
        dynamic_signatures::SPECS,
        ip_uncommon_protolist::SPECS,
        l4bdos_file_object::SPECS,
        network_whitelist::SPECS,
        profile::SPECS,
        udp_portlist::SPECS,
    ]
}
