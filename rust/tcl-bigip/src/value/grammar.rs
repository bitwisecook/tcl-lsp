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

use tcl_bigip_schema::{AddressGrammar, default_registry};

use super::{
    Address, BigIpName, Destination, FQDN, IPAddress, MemberKey, Network, Port, WildcardAddress,
};

/// Value returned by the registry-selected configuration reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationAddress<'a> {
    /// An IP literal or a component-specific wildcard.
    Address(Address),
    /// An IP network with its optional route domain.
    Network(Network),
    /// An endpoint, including its original service and separator.
    Destination(Destination),
    /// A list of literal IPs with optional route domains.
    Addresses(Vec<IPAddress>),
    /// A list of host endpoints with optional services.
    Endpoints(Vec<Destination>),
    /// A node and service in a pool member key.
    Member(MemberKey<'a>),
    /// A DNS host name.
    Fqdn(FQDN),
    /// Transport service and original spelling.
    Service(Port),
    /// Prefix interpretations by IP family.
    Mask {
        /// Prefix applicable to an IPv4 address.
        ipv4: Option<u8>,
        /// Prefix applicable to an IPv6 address.
        ipv6: Option<u8>,
    },
    /// Two object references in a GTM pool member key.
    GtmMember {
        /// GTM server reference.
        server: &'a str,
        /// Virtual server reference.
        virtual_server: &'a str,
    },
}

impl ConfigurationAddress<'_> {
    pub(crate) fn route_domain(&self) -> Option<u32> {
        match self {
            Self::Address(Address::Ip(address)) => address.route_domain,
            Self::Network(network) => network.route_domain,
            Self::Destination(destination) => {
                destination.route_domain.as_ref().map(|domain| domain.id)
            }
            _ => None,
        }
    }
}

pub(crate) fn read_address_property<'a>(
    module: &str,
    object_type: &str,
    property: &str,
    text: &'a str,
) -> Option<ConfigurationAddress<'a>> {
    read_address_field(module, object_type, &[property], text)
}

/// Read a TMSH field under its declared grammar, including nested properties.
#[must_use]
pub fn read_address_field<'a>(
    module: &str,
    object_type: &str,
    property: &[&str],
    text: &'a str,
) -> Option<ConfigurationAddress<'a>> {
    let field = default_registry().property(module, object_type, property)?;
    read_grammar(field.address_grammar?, text)
}

fn read_grammar(grammar: AddressGrammar, text: &str) -> Option<ConfigurationAddress<'_>> {
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
        AddressGrammar::Endpoint => {
            Destination::try_parse(text).map(ConfigurationAddress::Destination)
        }
        AddressGrammar::AddressList => crate::parser::helpers::parse_list_block(text)
            .into_iter()
            .map(|entry| IPAddress::try_parse(&entry))
            .collect::<Option<Vec<_>>>()
            .map(ConfigurationAddress::Addresses),
        AddressGrammar::EndpointList => crate::parser::helpers::parse_list_block(text)
            .into_iter()
            .map(|entry| Destination::try_parse(&entry))
            .collect::<Option<Vec<_>>>()
            .map(ConfigurationAddress::Endpoints),
        AddressGrammar::Mask => {
            let ipv4 = Network::prefix_from_mask(text, false);
            let ipv6 = Network::prefix_from_mask(text, true);
            (ipv4.is_some() || ipv6.is_some()).then_some(ConfigurationAddress::Mask { ipv4, ipv6 })
        }
        AddressGrammar::Fqdn => FQDN::try_parse(text).map(ConfigurationAddress::Fqdn),
        AddressGrammar::Service => Port::try_parse_service(text).map(ConfigurationAddress::Service),
        AddressGrammar::GtmMemberKey => {
            let (server, virtual_server) = text.split_once(':')?;
            let valid = |name: &str| {
                if name.starts_with('/') {
                    BigIpName::try_parse(name).is_some()
                } else {
                    BigIpName::valid_leaf(name)
                }
            };
            (valid(server) && valid(virtual_server)).then_some(ConfigurationAddress::GtmMember {
                server,
                virtual_server,
            })
        }
    }
}

/// Return every distinct reading when no field grammar is available.
#[must_use]
pub fn address_readings(text: &str) -> Vec<ConfigurationAddress<'_>> {
    let mut readings = Vec::new();
    for grammar in [
        AddressGrammar::AddressOrWildcard,
        AddressGrammar::Network,
        AddressGrammar::Endpoint,
        AddressGrammar::Destination,
        AddressGrammar::MemberKey,
        AddressGrammar::MonitorDestination,
        AddressGrammar::GtmMemberKey,
        AddressGrammar::Fqdn,
        AddressGrammar::Mask,
        AddressGrammar::Service,
    ] {
        if let Some(value) = read_grammar(grammar, text)
            && !readings.contains(&value)
        {
            readings.push(value);
        }
    }
    readings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_addresses_and_named_services_use_the_declared_grammar() {
        assert!(matches!(
            read_address_field("ltm", "node", &["fqdn", "name"], "xn--caf-dma.example"),
            Some(ConfigurationAddress::Fqdn(_)),
        ));
        assert!(matches!(
            read_address_field("ltm", "pool", &["members", "address"], "2001:db8::53%0"),
            Some(ConfigurationAddress::Address(Address::Ip(address))) if address.route_domain == Some(0),
        ));
        assert!(matches!(
            read_address_field("ltm", "virtual", &["mask"], "255.255.255.0"),
            Some(ConfigurationAddress::Mask {
                ipv4: Some(24),
                ipv6: None
            }),
        ));
        assert!(matches!(
            read_address_field(
                "gtm",
                "pool a",
                &["members"],
                "/Common/server:/Common/virtual"
            ),
            Some(ConfigurationAddress::GtmMember {
                server: "/Common/server",
                virtual_server: "/Common/virtual"
            }),
        ));
        let readings = address_readings("192.0.2.53%0");
        assert!(
            readings
                .iter()
                .any(|value| matches!(value, ConfigurationAddress::Address(_)))
        );
        assert!(
            readings
                .iter()
                .any(|value| matches!(value, ConfigurationAddress::Network(_)))
        );
        assert!(address_readings("https").iter().any(|value| {
            matches!(value, ConfigurationAddress::Service(service) if service.port == 443)
        }));
    }

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
