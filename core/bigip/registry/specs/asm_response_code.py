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
            "asm_response_code",
            module="asm",
            object_types=("response-code",),
        ),
        header_types=(("asm", "response-code"),),
        properties=(),
    )
