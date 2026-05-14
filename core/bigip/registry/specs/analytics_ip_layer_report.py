from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "analytics_ip_layer_report",
            module="analytics",
            object_types=("ip-layer report",),
        ),
        header_types=(("analytics", "ip-layer report"),),
        properties=(),
    )
