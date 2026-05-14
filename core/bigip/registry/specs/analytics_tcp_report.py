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
            "analytics_tcp_report",
            module="analytics",
            object_types=("tcp report",),
        ),
        header_types=(("analytics", "tcp report"),),
        properties=(),
    )
