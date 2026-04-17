from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_availability",
            module="sys",
            object_types=("availability",),
        ),
        header_types=(("sys", "availability"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
