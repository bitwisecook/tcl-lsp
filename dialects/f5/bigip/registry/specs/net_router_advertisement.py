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
            "net_router_advertisement",
            module="net",
            object_types=("router-advertisement",),
        ),
        header_types=(("net", "router-advertisement"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="autonomous", value_type="unknown", default="1"),
            BigipPropertySpec(name="current-hop-limit", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disabled", value_type="unknown"),
            BigipPropertySpec(name="max-interval", value_type="integer", default="600"),
            BigipPropertySpec(name="min-interval", value_type="integer", default="200"),
            BigipPropertySpec(name="mtu", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="no-other-config", value_type="unknown", default="0 zero"),
            BigipPropertySpec(name="on-link", value_type="unknown", default="1"),
            BigipPropertySpec(name="preferred-lifetime", value_type="integer", default="604800"),
            BigipPropertySpec(name="prefix", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="prefix-length", value_type="integer"),
            BigipPropertySpec(name="prefixes", value_type="unknown"),
            BigipPropertySpec(name="reachable-time", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="retransmit-timer", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="router-lifetime", value_type="integer", default="1800"),
            BigipPropertySpec(name="unmanaged", value_type="unknown", default="0 (zero)"),
            BigipPropertySpec(name="valid-lifetime", value_type="integer", default="2592000"),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                references=("net_vlan", "net_vlan_allowed", "net_vlan_group"),
            ),
        ),
    )
