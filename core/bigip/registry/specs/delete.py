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
            "delete",
            module="delete",
            object_types=("",),
        ),
        header_types=(("delete", ""),),
        properties=(),
    )
