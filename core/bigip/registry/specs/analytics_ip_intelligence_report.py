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
            "analytics_ip_intelligence_report",
            module="analytics",
            object_types=("ip-intelligence report",),
        ),
        header_types=(("analytics", "ip-intelligence report"),),
        properties=(),
    )
