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
            "ltm_profile_nat_stats",
            module="ltm",
            object_types=("profile nat-stats",),
        ),
        header_types=(("ltm", "profile nat-stats"),),
        properties=(),
    )
