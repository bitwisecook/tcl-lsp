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
            "net_cos_map_8021p",
            module="net",
            object_types=("cos map-8021p",),
        ),
        header_types=(("net", "cos map-8021p"),),
        properties=(
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="traffic-priority", value_type="string"),
        ),
    )
