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
            "vcmp_health_prompt",
            module="vcmp",
            object_types=("health prompt",),
        ),
        header_types=(("vcmp", "health prompt"),),
        properties=(),
    )
