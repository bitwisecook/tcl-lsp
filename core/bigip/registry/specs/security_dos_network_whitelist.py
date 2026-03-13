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
            BigipPropertySpec(name="address-list", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="entries",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("destination",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("destination",),
                min_value=0,
                max_value=65535,
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
            BigipPropertySpec(name="source", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="address",
                value_type="string",
                in_sections=("source",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="vlans",
                value_type="reference",
                in_sections=("source",),
                references=("net_vlan",),
            ),
            BigipPropertySpec(
                name="extended-entries",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="description", value_type="string", in_sections=("extended-entries",)
            ),
            BigipPropertySpec(
                name="destination", value_type="string", in_sections=("extended-entries",)
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
                name="source", value_type="string", in_sections=("extended-entries",)
            ),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(name="entries", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="internal-net", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="source", value_type="string", in_sections=("internal-net",)),
            BigipPropertySpec(
                name="extended-entries", value_type="string", in_sections=("security",)
            ),
            BigipPropertySpec(name="netwl", value_type="string", in_sections=("extended-entries",)),
            BigipPropertySpec(
                name="description", value_type="boolean", in_sections=("netwl",), allow_none=True
            ),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("netwl",)),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("netwl",)),
            BigipPropertySpec(name="source", value_type="string", in_sections=("netwl",)),
        ),
    )
