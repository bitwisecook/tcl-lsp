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
            "security_datasync_device_stats",
            module="security",
            object_types=("datasync device-stats",),
        ),
        header_types=(("security", "datasync device-stats"),),
    )
