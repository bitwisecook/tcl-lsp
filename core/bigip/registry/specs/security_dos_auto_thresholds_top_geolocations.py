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
            "security_dos_auto_thresholds_top_geolocations",
            module="security",
            object_types=("dos auto-thresholds top-geolocations",),
        ),
        header_types=(("security", "dos auto-thresholds top-geolocations"),),
    )
