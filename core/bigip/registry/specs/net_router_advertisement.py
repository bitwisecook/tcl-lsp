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
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="autonomous", value_type="unknown"),
            BigipPropertySpec(name="current-hop-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disabled", value_type="unknown"),
            BigipPropertySpec(name="max-interval", value_type="integer"),
            BigipPropertySpec(name="min-interval", value_type="integer"),
            BigipPropertySpec(name="mtu", value_type="integer"),
            BigipPropertySpec(name="no-other-config", value_type="unknown"),
            BigipPropertySpec(name="on-link", value_type="unknown"),
            BigipPropertySpec(name="preferred-lifetime", value_type="integer"),
            BigipPropertySpec(name="prefix", value_type="string"),
            BigipPropertySpec(name="prefix-length", value_type="integer"),
            BigipPropertySpec(name="prefixes", value_type="unknown"),
            BigipPropertySpec(name="reachable-time", value_type="integer"),
            BigipPropertySpec(name="retransmit-timer", value_type="integer"),
            BigipPropertySpec(name="router-lifetime", value_type="integer"),
            BigipPropertySpec(name="unmanaged", value_type="unknown"),
            BigipPropertySpec(name="valid-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                references=("net_vlan", "net_vlan_allowed", "net_vlan_group"),
            ),
        ),
    )
