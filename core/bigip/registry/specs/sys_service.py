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
            "sys_service",
            module="sys",
            object_types=("service",),
        ),
        header_types=(("sys", "service"),),
        properties=(
            BigipPropertySpec(name="restart", value_type="string"),
            BigipPropertySpec(name="start", value_type="string"),
            BigipPropertySpec(name="stop", value_type="string"),
        ),
    )
