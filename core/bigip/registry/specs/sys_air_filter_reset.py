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
            "sys_air_filter_reset",
            module="sys",
            object_types=("air-filter-reset",),
        ),
        header_types=(("sys", "air-filter-reset"),),
    )
