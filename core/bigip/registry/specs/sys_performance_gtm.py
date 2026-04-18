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
            "sys_performance_gtm",
            module="sys",
            object_types=("performance gtm",),
        ),
        header_types=(("sys", "performance gtm"),),
    )
