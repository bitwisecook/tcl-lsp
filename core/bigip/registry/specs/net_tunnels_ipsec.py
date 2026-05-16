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
            "net_tunnels_ipsec",
            module="net",
            object_types=("tunnels ipsec",),
        ),
        header_types=(("net", "tunnels ipsec"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("net_tunnels_ipsec",),
                default="ipsec",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="traffic-selector",
                value_type="reference",
                references=("net_ipsec_traffic_selector",),
            ),
        ),
    )
