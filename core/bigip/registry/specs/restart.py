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
            "restart",
            module="restart",
            object_types=("",),
        ),
        header_types=(("restart", ""),),
        properties=(),
    )
