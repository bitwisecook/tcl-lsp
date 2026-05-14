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
            "vcmp_health_software",
            module="vcmp",
            object_types=("health software",),
        ),
        header_types=(("vcmp", "health software"),),
        properties=(),
    )
