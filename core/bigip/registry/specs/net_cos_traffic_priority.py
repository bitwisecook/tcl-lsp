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
            "net_cos_traffic_priority",
            module="net",
            object_types=("cos traffic-priority",),
        ),
        header_types=(("net", "cos traffic-priority"),),
        properties=(
            BigipPropertySpec(name="weight", value_type="string"),
            BigipPropertySpec(name="buffer", value_type="string"),
        ),
    )
