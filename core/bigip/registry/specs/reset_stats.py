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
            "reset_stats",
            module="reset-stats",
            object_types=("",),
        ),
        header_types=(("reset-stats", ""),),
        properties=(),
    )
