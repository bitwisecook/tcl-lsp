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
            "analytics_cpu_per_vip_report",
            module="analytics",
            object_types=("cpu-per-vip report",),
        ),
        header_types=(("analytics", "cpu-per-vip report"),),
        properties=(),
    )
