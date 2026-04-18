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
            "security_dos_spva_stats",
            module="security",
            object_types=("dos spva-stats",),
        ),
        header_types=(("security", "dos spva-stats"),),
    )
