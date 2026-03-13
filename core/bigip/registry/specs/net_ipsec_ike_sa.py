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
            "net_ipsec_ike_sa",
            module="net",
            object_types=("ipsec ike-sa",),
        ),
        header_types=(("net", "ipsec ike-sa"),),
        properties=(
            BigipPropertySpec(name="peer-ip", value_type="string"),
            BigipPropertySpec(name="peer-name", value_type="string"),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="traffic-selector", value_type="string"),
        ),
    )
