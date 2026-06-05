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
            "sys_software_update",
            module="sys",
            object_types=("software update",),
        ),
        header_types=(("sys", "software update"),),
        properties=(
            BigipPropertySpec(name="auto-check", value_type="unknown"),
            BigipPropertySpec(name="auto-phonehome", value_type="unknown"),
            BigipPropertySpec(name="frequency", value_type="unknown"),
        ),
    )
