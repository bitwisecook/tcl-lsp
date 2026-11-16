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

//! BIG-IP object specs for `apm aaa okta-connector`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_okta_connector",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa okta-connector"],
    },
    header_types: &[("apm", "aaa okta-connector")],
    properties: &[
        BigipPropertySpec {
            name: "domain",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "token",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transport",
            value_type: ValueKind::Reference,
            required: true,
            references: &[
                "apm_aaa_http_connector_transport",
                "ltm_message_routing_diameter_transport_config",
                "ltm_message_routing_generic_transport_config",
                "ltm_message_routing_mqtt_transport_config",
                "ltm_message_routing_sip_transport_config",
            ],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
