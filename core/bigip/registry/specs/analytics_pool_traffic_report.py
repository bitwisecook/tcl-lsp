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
            "analytics_pool_traffic_report",
            module="analytics",
            object_types=("pool-traffic report",),
        ),
        header_types=(("analytics", "pool-traffic report"),),
        properties=(),
    )
