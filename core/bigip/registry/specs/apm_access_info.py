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
            "apm_access_info",
            module="apm",
            object_types=("access-info",),
        ),
        header_types=(("apm", "access-info"),),
        properties=(),
    )
