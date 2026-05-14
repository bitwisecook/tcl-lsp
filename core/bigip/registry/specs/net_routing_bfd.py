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
            "net_routing_bfd",
            module="net",
            object_types=("routing bfd",),
        ),
        header_types=(("net", "routing bfd"),),
        properties=(
            BigipPropertySpec(name="gtsm", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="gtsm-ttl", value_type="integer"),
            BigipPropertySpec(
                name="multihop-peer",
                value_type="reference",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="interval",
                value_type="boolean",
                in_sections=("multihop-peer",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="minrx",
                value_type="boolean",
                in_sections=("multihop-peer",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="multiplier",
                value_type="boolean",
                in_sections=("multihop-peer",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="notification",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="slow-timer", value_type="integer"),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                references=("net_vlan",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("vlan",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="interval",
                value_type="boolean",
                in_sections=("vlan",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="minrx",
                value_type="boolean",
                in_sections=("vlan",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="multiplier",
                value_type="boolean",
                in_sections=("vlan",),
                allow_none=True,
            ),
        ),
    )
