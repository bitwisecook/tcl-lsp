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
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(name="destination", value_type="unknown", in_sections=("entries",)),
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
            ),
            BigipPropertySpec(name="source", value_type="unknown", in_sections=("entries",)),
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
            ),
            BigipPropertySpec(
                name="source",
                value_type="unknown",
                in_sections=("extended-entries",),
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
