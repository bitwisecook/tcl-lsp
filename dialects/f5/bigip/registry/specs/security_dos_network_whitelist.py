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
            "security_dos_network_whitelist",
            module="security",
            object_types=("dos network-whitelist",),
        ),
        header_types=(("security", "dos network-whitelist"),),
        properties=(
            BigipPropertySpec(
                name="address-list",
                value_type="reference",
                references=("net_address_list", "security_firewall_address_list"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="entries",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("entries",),
                    ),
                    BigipPropertySpec(
                        name="destination",
                        value_type="unknown",
                        in_sections=("entries",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="ip-protocol",
                        value_type="enum",
                        in_sections=("entries",),
                        enum_values=("any", "icmp", "igmp", "tcp", "udp"),
                    ),
                    BigipPropertySpec(
                        name="match-ip-version",
                        value_type="enum",
                        in_sections=("entries",),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                    BigipPropertySpec(
                        name="source",
                        value_type="unknown",
                        in_sections=("entries",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                in_sections=("entries",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="unknown",
                        in_sections=("entries", "destination"),
                    ),
                    BigipPropertySpec(
                        name="port",
                        value_type="unknown",
                        in_sections=("entries", "destination"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="unknown",
                in_sections=("entries", "destination"),
            ),
            BigipPropertySpec(
                name="port",
                value_type="unknown",
                in_sections=("entries", "destination"),
            ),
            BigipPropertySpec(
                name="ip-protocol",
                value_type="enum",
                in_sections=("entries",),
                enum_values=("any", "icmp", "igmp", "tcp", "udp"),
            ),
            BigipPropertySpec(
                name="match-ip-version",
                value_type="enum",
                in_sections=("entries",),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="source",
                value_type="unknown",
                in_sections=("entries",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="unknown",
                        in_sections=("entries", "source"),
                    ),
                    BigipPropertySpec(
                        name="vlans",
                        value_type="enum",
                        in_sections=("entries", "source"),
                        enum_values=("vlanid/mask",),
                        references=(
                            "net_fdb_vlan",
                            "net_vlan",
                            "net_vlan_allowed",
                            "net_vlan_group",
                            "sys_sflow_data_source_vlan",
                            "sys_sflow_global_settings_vlan",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="unknown",
                in_sections=("entries", "source"),
            ),
            BigipPropertySpec(
                name="vlans",
                value_type="enum",
                in_sections=("entries", "source"),
                enum_values=("vlanid/mask",),
                references=(
                    "net_fdb_vlan",
                    "net_vlan",
                    "net_vlan_allowed",
                    "net_vlan_group",
                    "sys_sflow_data_source_vlan",
                    "sys_sflow_global_settings_vlan",
                ),
            ),
            BigipPropertySpec(
                name="extended-entries",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("extended-entries",),
                    ),
                    BigipPropertySpec(
                        name="destination",
                        value_type="unknown",
                        in_sections=("extended-entries",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="ip-protocol",
                        value_type="enum",
                        in_sections=("extended-entries",),
                        enum_values=("any", "icmp", "igmp", "tcp", "udp"),
                    ),
                    BigipPropertySpec(
                        name="match-ip-version",
                        value_type="enum",
                        in_sections=("extended-entries",),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                    BigipPropertySpec(
                        name="source",
                        value_type="unknown",
                        in_sections=("extended-entries",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                in_sections=("extended-entries",),
            ),
            BigipPropertySpec(
                name="destination",
                value_type="unknown",
                in_sections=("extended-entries",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="unknown",
                        in_sections=("extended-entries", "destination"),
                    ),
                    BigipPropertySpec(
                        name="port",
                        value_type="unknown",
                        in_sections=("extended-entries", "destination"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="unknown",
                in_sections=("extended-entries", "destination"),
            ),
            BigipPropertySpec(
                name="port",
                value_type="unknown",
                in_sections=("extended-entries", "destination"),
            ),
            BigipPropertySpec(
                name="ip-protocol",
                value_type="enum",
                in_sections=("extended-entries",),
                enum_values=("any", "icmp", "igmp", "tcp", "udp"),
            ),
            BigipPropertySpec(
                name="match-ip-version",
                value_type="enum",
                in_sections=("extended-entries",),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="source",
                value_type="unknown",
                in_sections=("extended-entries",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="unknown",
                        in_sections=("extended-entries", "source"),
                    ),
                    BigipPropertySpec(
                        name="vlans",
                        value_type="enum",
                        in_sections=("extended-entries", "source"),
                        enum_values=("vlanid/mask",),
                        references=(
                            "net_fdb_vlan",
                            "net_vlan",
                            "net_vlan_allowed",
                            "net_vlan_group",
                            "sys_sflow_data_source_vlan",
                            "sys_sflow_global_settings_vlan",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="unknown",
                in_sections=("extended-entries", "source"),
            ),
            BigipPropertySpec(
                name="vlans",
                value_type="enum",
                in_sections=("extended-entries", "source"),
                enum_values=("vlanid/mask",),
                references=(
                    "net_fdb_vlan",
                    "net_vlan",
                    "net_vlan_allowed",
                    "net_vlan_group",
                    "sys_sflow_data_source_vlan",
                    "sys_sflow_global_settings_vlan",
                ),
            ),
        ),
    )
