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
            "analytics_tcp_analytics_report",
            module="analytics",
            object_types=("tcp-analytics report",),
        ),
        header_types=(("analytics", "tcp-analytics report"),),
        properties=(),
    )
