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
            "analytics_udp_report",
            module="analytics",
            object_types=("udp report",),
        ),
        header_types=(("analytics", "udp report"),),
        properties=(),
    )
