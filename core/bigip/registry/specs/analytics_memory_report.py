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
            "analytics_memory_report",
            module="analytics",
            object_types=("memory report",),
        ),
        header_types=(("analytics", "memory report"),),
        properties=(),
    )
