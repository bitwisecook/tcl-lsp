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

use tcl_bigip::{parser::parse_bigip_conf, wireshark_profile::build_wireshark_profile};

#[test]
fn irule_services_read_literal_endpoints_with_route_domains() {
    crate::install();
    let source = "ltm rule /Common/r { when CLIENT_ACCEPTED { node 192.0.2.53:80; pool /Common/probe member 2001:db8::53%2 443; snat 192.0.2.54%2 8080 } }";
    let profile =
        build_wireshark_profile(&[(parse_bigip_conf(source, "Common"), source.to_owned())]);
    let ports: Vec<_> = profile
        .services
        .iter()
        .filter(|(name, _, _)| name.contains("irule"))
        .map(|(_, port, _)| *port)
        .collect();
    assert_eq!(ports, [80, 443, 8080]);
}
