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
            "submit",
            module="submit",
            object_types=("",),
        ),
        header_types=(("submit", ""),),
        properties=(),
    )
