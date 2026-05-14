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
            "analytics_dos_vis_common_report",
            module="analytics",
            object_types=("dos-vis-common report",),
        ),
        header_types=(("analytics", "dos-vis-common report"),),
        properties=(),
    )
