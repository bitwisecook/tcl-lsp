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
            "analytics_memory_per_process_report",
            module="analytics",
            object_types=("memory-per-process report",),
        ),
        header_types=(("analytics", "memory-per-process report"),),
        properties=(),
    )
