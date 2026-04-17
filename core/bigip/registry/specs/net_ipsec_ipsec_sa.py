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
            "net_ipsec_ipsec_sa",
            module="net",
            object_types=("ipsec ipsec-sa",),
        ),
        header_types=(("net", "ipsec ipsec-sa"),),
        properties=(
            BigipPropertySpec(name="src-addr", value_type="string"),
            BigipPropertySpec(name="dst-addr", value_type="string"),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="spi", value_type="integer"),
            BigipPropertySpec(name="traffic-selector", value_type="string"),
        ),
    )
