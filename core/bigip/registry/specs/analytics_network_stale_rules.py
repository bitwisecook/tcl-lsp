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
            "analytics_network_stale_rules",
            module="analytics",
            object_types=("network stale-rules",),
        ),
        header_types=(("analytics", "network stale-rules"),),
        properties=(),
    )
