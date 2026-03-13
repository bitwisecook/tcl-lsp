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
            BigipPropertySpec(name="current-hop-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disabled", value_type="boolean"),
            BigipPropertySpec(name="max-interval", value_type="integer"),
            BigipPropertySpec(name="min-interval", value_type="integer"),
            BigipPropertySpec(name="mtu", value_type="integer"),
            BigipPropertySpec(name="no-other-config", value_type="string"),
            BigipPropertySpec(name="autonomous", value_type="boolean"),
            BigipPropertySpec(name="on-link", value_type="boolean"),
            BigipPropertySpec(name="preferred-lifetime", value_type="integer"),
            BigipPropertySpec(name="prefix", value_type="string"),
            BigipPropertySpec(name="prefix-length", value_type="integer"),
            BigipPropertySpec(name="valid-lifetime", value_type="integer"),
            BigipPropertySpec(name="reachable-time", value_type="integer"),
            BigipPropertySpec(name="retransmit-timer", value_type="integer"),
            BigipPropertySpec(name="router-lifetime", value_type="integer"),
            BigipPropertySpec(name="unmanaged", value_type="string"),
            BigipPropertySpec(name="vlan", value_type="reference", references=("net_vlan",)),
        ),
    )
