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
            "net_ipsec_traffic_selector",
            module="net",
            object_types=("ipsec traffic-selector",),
        ),
        header_types=(("net", "ipsec traffic-selector"),),
        properties=(
            BigipPropertySpec(name="action", value_type="unknown"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="destination-port", value_type="unknown"),
            BigipPropertySpec(
                name="direction",
                value_type="enum",
                enum_values=("both", "in", "out"),
                default="both",
            ),
            BigipPropertySpec(name="ip-protocol", value_type="unknown"),
            BigipPropertySpec(
                name="ipsec-policy",
                value_type="reference",
                references=("net_ipsec_ipsec_policy",),
            ),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="source-port", value_type="unknown"),
        ),
    )
