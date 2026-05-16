from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "asm_httpclass_asm",
            module="asm",
            object_types=("httpclass-asm",),
        ),
        header_types=(("asm", "httpclass-asm"),),
        properties=(
            BigipPropertySpec(
                name="active-policy-name",
                value_type="string",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="language", value_type="unknown"),
            BigipPropertySpec(name="predefined-policy", value_type="unknown"),
        ),
    )
