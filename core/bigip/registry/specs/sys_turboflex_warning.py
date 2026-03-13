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
            "sys_turboflex_warning",
            module="sys",
            object_types=("turboflex warning",),
        ),
        header_types=(("sys", "turboflex warning"),),
    )
