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
            "security_dos_auto_thresholds_heavy_urls",
            module="security",
            object_types=("dos auto-thresholds heavy-urls",),
        ),
        header_types=(("security", "dos auto-thresholds heavy-urls"),),
    )
