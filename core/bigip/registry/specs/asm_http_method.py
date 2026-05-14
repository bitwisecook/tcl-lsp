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
            "asm_http_method",
            module="asm",
            object_types=("http-method",),
        ),
        header_types=(("asm", "http-method"),),
        properties=(),
    )
