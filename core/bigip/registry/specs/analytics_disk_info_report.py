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
            "analytics_disk_info_report",
            module="analytics",
            object_types=("disk-info report",),
        ),
        header_types=(("analytics", "disk-info report"),),
        properties=(),
    )
