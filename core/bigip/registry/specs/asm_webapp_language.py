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
            "asm_webapp_language",
            module="asm",
            object_types=("webapp-language",),
        ),
        header_types=(("asm", "webapp-language"),),
        properties=(),
    )
