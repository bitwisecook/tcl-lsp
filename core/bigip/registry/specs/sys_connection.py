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
            "sys_connection",
            module="sys",
            object_types=("connection",),
        ),
        header_types=(("sys", "connection"),),
        properties=(
            BigipPropertySpec(name="flow-accel-type", value_type="unknown"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
        ),
    )
