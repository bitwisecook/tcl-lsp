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
            "sys_provision",
            module="sys",
            object_types=("provision",),
        ),
        header_types=(("sys", "provision"),),
        properties=(
            BigipPropertySpec(name="cpu-ratio", value_type="integer", default="none"),
            BigipPropertySpec(name="disk-ratio", value_type="integer", default="none"),
            BigipPropertySpec(
                name="level",
                value_type="enum",
                allow_none=True,
                enum_values=("custom", "dedicated", "minimum", "nominal", "none"),
            ),
            BigipPropertySpec(name="memory-ratio", value_type="integer", default="none"),
        ),
    )
