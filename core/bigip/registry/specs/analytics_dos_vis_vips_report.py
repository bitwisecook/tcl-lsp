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
            "analytics_dos_vis_vips_report",
            module="analytics",
            object_types=("dos-vis-vips report",),
        ),
        header_types=(("analytics", "dos-vis-vips report"),),
        properties=(),
    )
