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
            "sys_performance_throughput",
            module="sys",
            object_types=("performance throughput",),
        ),
        header_types=(("sys", "performance throughput"),),
    )
