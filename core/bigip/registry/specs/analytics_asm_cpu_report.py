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
            "analytics_asm_cpu_report",
            module="analytics",
            object_types=("asm-cpu report",),
        ),
        header_types=(("analytics", "asm-cpu report"),),
        properties=(),
    )
