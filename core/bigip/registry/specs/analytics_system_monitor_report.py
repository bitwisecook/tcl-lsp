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
            "analytics_system_monitor_report",
            module="analytics",
            object_types=("system-monitor report",),
        ),
        header_types=(("analytics", "system-monitor report"),),
        properties=(),
    )
