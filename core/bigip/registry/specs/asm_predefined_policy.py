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
            "asm_predefined_policy",
            module="asm",
            object_types=("predefined-policy",),
        ),
        header_types=(("asm", "predefined-policy"),),
        properties=(),
    )
