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
            BigipPropertySpec(name="action", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="destination-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="direction", value_type="enum", enum_values=("both", "in", "out")
            ),
            BigipPropertySpec(name="ipsec-policy", value_type="string"),
            BigipPropertySpec(name="ip-protocol", value_type="string"),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="source-port", value_type="integer", min_value=0, max_value=65535
            ),
        ),
    )
