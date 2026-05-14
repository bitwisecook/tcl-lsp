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
            "net_bwc_traffic_group",
            module="net",
            object_types=("bwc traffic-group",),
        ),
        header_types=(("net", "bwc traffic-group"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dynamic", value_type="unknown"),
            BigipPropertySpec(name="priority-classes", value_type="list"),
            BigipPropertySpec(
                name="weight-percentage",
                value_type="integer",
                in_sections=("priority-classes",),
            ),
        ),
    )
