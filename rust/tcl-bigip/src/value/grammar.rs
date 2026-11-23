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

//! Read address properties with the grammar declared by the BIG-IP registry.

use tcl_registry::bigip::{AddressGrammar, default_registry};

use super::{Address, Destination, IPAddress, MemberKey, Network, WildcardAddress};

/// Value returned by the registry-selected configuration reader.
pub(crate) enum ConfigurationAddress<'a> {
    Address(Address),
    Network(Network),
    Destination(Destination),
    Member(MemberKey<'a>),
}

pub(crate) fn read_address_property<'a>(
    module: &str,
    object_type: &str,
    property: &str,
    text: &'a str,
) -> Option<ConfigurationAddress<'a>> {
    let grammar = default_registry().address_grammar(module, object_type, property)?;
    match grammar {
        AddressGrammar::Address => IPAddress::try_parse(text)
            .map(|address| ConfigurationAddress::Address(Address::Ip(address))),
        AddressGrammar::AddressOrWildcard => WildcardAddress::try_parse(text)
            .map(Address::Wildcard)
            .or_else(|| IPAddress::try_parse(text).map(Address::Ip))
            .map(ConfigurationAddress::Address),
        AddressGrammar::Network => Network::try_parse(text).map(ConfigurationAddress::Network),
        AddressGrammar::Destination => {
            Destination::try_parse_tmsh(text).map(ConfigurationAddress::Destination)
        }
        AddressGrammar::MemberKey => MemberKey::try_parse(text).map(ConfigurationAddress::Member),
        AddressGrammar::MonitorDestination => {
            Destination::try_parse_tmsh_monitor(text).map(ConfigurationAddress::Destination)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_address_grammars_follow_measured_21_1_rows() {
        for (object, property, value, accepted) in [
            ("virtual", "destination", "2001:db8::53%0.80", true),
            ("virtual", "destination", "2001:db8::53%0:80", false),
            ("virtual", "destination", "2001:db8::53.http", true),
            ("virtual", "destination", "/Common/alias:80", true),
            ("virtual", "destination", "alias:80", true),
            ("virtual", "destination", "[2001:db8::53]:80", true),
            ("virtual", "destination", "2001:db8::53:80", false),
            ("virtual", "destination", "any6.any", true),
            ("virtual", "destination", "any6", false),
            ("virtual", "destination", "*.*", false),
            ("monitor http", "destination", "2001:db8::53%0:80", true),
            ("monitor http", "destination", "*.*", true),
            ("monitor http", "destination", "2001:db8::53.*", false),
            ("pool", "members", "/Common/node:80", true),
            ("pool", "members", "/Common/node.80", false),
            ("pool", "members", "2001:db8::53%0:80", false),
            ("pool", "members", "2001:db8::53%0.80", true),
        ] {
            assert_eq!(
                read_address_property("ltm", object, property, value).is_some(),
                accepted,
                "{object} {property} {value}"
            );
        }
        assert!(matches!(
            read_address_property("net", "route", "network", "default%0"),
            Some(ConfigurationAddress::Network(network)) if network.route_domain == Some(0)
        ));
    }
}
