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
            "security_blacklist_publisher_profile",
            module="security",
            object_types=("blacklist-publisher profile",),
        ),
        header_types=(("security", "blacklist-publisher profile"),),
        properties=(
            BigipPropertySpec(
                name="bgp-flowspec-advertisement-action",
                value_type="enum",
                enum_values=("drop", "rate-limit", "qos"),
            ),
            BigipPropertySpec(name="bgp-flowspec-dscp-value", value_type="integer"),
            BigipPropertySpec(name="bgp-flowspec-rate-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(
                name="traffic-group", value_type="reference", references=("cm_traffic_group",)
            ),
            BigipPropertySpec(name="route-advertisement-nexthop", value_type="string"),
            BigipPropertySpec(name="route-advertisement-nexthop-v6", value_type="string"),
        ),
    )
