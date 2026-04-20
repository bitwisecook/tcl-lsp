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
            "security_nat_policy",
            module="security",
            object_types=("nat policy",),
        ),
        header_types=(("security", "nat policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="rules",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(name="ip-protocol", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="log-profile", value_type="boolean", in_sections=("rules",), allow_none=True
            ),
            BigipPropertySpec(
                name="place-after",
                value_type="reference",
                in_sections=("rules",),
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="place-before",
                value_type="reference",
                in_sections=("rules",),
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="status",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="address-lists",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="port-lists",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                in_sections=("destination",),
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="proxy-arp",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="route-advertisement",
                value_type="enum",
                in_sections=("destination",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="source", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="address-lists",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="port-lists",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="vlans",
                value_type="reference",
                in_sections=("source",),
                enum_values=("add", "default", "delete", "replace-all-with"),
                references=("net_vlan",),
            ),
            BigipPropertySpec(name="translation", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="destination",
                value_type="boolean",
                in_sections=("translation",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="source", value_type="boolean", in_sections=("translation",), allow_none=True
            ),
            BigipPropertySpec(name="next-hop", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="gw",
                value_type="reference",
                in_sections=("next-hop",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
                references=("net_self", "net_route", "ltm_virtual_address"),
            ),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("next-hop",),
                allow_none=True,
                references=("net_vlan",),
            ),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                in_sections=("next-hop",),
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="type",
                value_type="reference",
                in_sections=("next-hop",),
                enum_values=("default", "pool", "gateway", "vlan"),
                references=("net_vlan",),
            ),
        ),
    )
