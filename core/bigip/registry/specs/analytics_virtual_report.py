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
            "analytics_virtual_report",
            module="analytics",
            object_types=("virtual report",),
        ),
        header_types=(("analytics", "virtual report"),),
        properties=(),
    )
