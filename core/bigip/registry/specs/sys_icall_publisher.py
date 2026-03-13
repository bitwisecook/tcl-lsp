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
            "sys_icall_publisher",
            module="sys",
            object_types=("icall publisher",),
        ),
        header_types=(("sys", "icall publisher"),),
    )
