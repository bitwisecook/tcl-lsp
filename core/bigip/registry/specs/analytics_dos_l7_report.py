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
            "analytics_dos_l7_report",
            module="analytics",
            object_types=("dos-l7 report",),
        ),
        header_types=(("analytics", "dos-l7 report"),),
        properties=(),
    )
