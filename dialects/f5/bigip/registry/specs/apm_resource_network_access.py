# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "apm_resource_network_access",
            module="apm",
            object_types=("resource network-access",),
        ),
        header_types=(("apm", "resource network-access"),),
        properties=(
            BigipPropertySpec(
                name="address-space-dhcp-requests-excluded",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="address-space-exclude",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="address-space-exclude-dns-name",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="address-space-exclude-subnet",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="address-space-include",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="address-space-include-dns-name",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="address-space-include-subnet",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="address-space-loc-dns-servers-excluded",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="address-space-local-subnets-excluded",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="address-space-protect",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="application-launch",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="application-launch-warning",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="auto-launch",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="client-interface-speed",
                value_type="integer",
                allow_none=True,
                default="100000000",
            ),
            BigipPropertySpec(
                name="client-ip-filter-engine",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="<false>",
            ),
            BigipPropertySpec(
                name="client-power-management",
                value_type="enum",
                enum_values=("ignore", "prevent", "terminate"),
            ),
            BigipPropertySpec(
                name="client-proxy",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(name="client-proxy-address", value_type="unknown", default="any6"),
            BigipPropertySpec(
                name="client-proxy-enforce-subnets",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="client-proxy-exclusion-list",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="client-proxy-ignore-auto-config-error",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="client-proxy-local-bypass",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="client-proxy-port",
                value_type="integer",
                allow_none=True,
                default="0 (zero)",
            ),
            BigipPropertySpec(
                name="client-proxy-script",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="client-proxy-use-http-pac",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="client-proxy-use-local-proxy",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="client-traffic-classifier",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="compression",
                value_type="enum",
                allow_none=True,
                enum_values=("gzip", "none"),
                default="none",
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="dns-primary", value_type="unknown", default="any6"),
            BigipPropertySpec(name="dns-secondary", value_type="unknown", default="any6"),
            BigipPropertySpec(
                name="dns-suffix",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="drive-mapping",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="dtls",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="dtls-port",
                value_type="integer",
                allow_none=True,
                default="4433",
            ),
            BigipPropertySpec(
                name="execute-logoff-scripts",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="idle-timeout-threshold",
                value_type="integer",
                allow_none=True,
                default="0 (zero)",
            ),
            BigipPropertySpec(
                name="idle-timeout-window",
                value_type="integer",
                allow_none=True,
                default="0 (zero)",
            ),
            BigipPropertySpec(
                name="ipv6-address-space-exclude-subnet",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ipv6-address-space-include-subnet",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="ipv6-dns-primary", value_type="unknown", default="any6"),
            BigipPropertySpec(name="ipv6-dns-secondary", value_type="unknown", default="any6"),
            BigipPropertySpec(
                name="ipv6-leasepool-name",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="leasepool-name",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="microsoft-network-client",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="microsoft-network-server",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="network-tunnel",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="optimized-app",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="provide-client-cert",
                value_type="enum",
                required=True,
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="proxy-arp",
                value_type="enum",
                required=True,
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="split-tunneling",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="static-host",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="supported-ip-version",
                value_type="enum",
                enum_values=("ipv4", "ipv4-ipv6"),
                shape_kind="ip-address",
                default="ipv4",
            ),
            BigipPropertySpec(
                name="sync-with-active-directory",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "app-tunnel",
                    "last",
                    "network-access",
                    "remote-desktop",
                    "web-application",
                ),
                default="network-access",
            ),
            BigipPropertySpec(name="wins-primary", value_type="unknown", default="any6"),
            BigipPropertySpec(name="wins-secondary", value_type="unknown", default="any6"),
        ),
    )
