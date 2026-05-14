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
            "sys_performance_all_stats",
            module="sys",
            object_types=("performance all-stats",),
        ),
        header_types=(("sys", "performance all-stats"),),
        properties=(),
    )
