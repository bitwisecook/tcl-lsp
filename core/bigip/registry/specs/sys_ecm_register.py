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
            "sys_ecm_register",
            module="sys",
            object_types=("ecm register",),
        ),
        header_types=(("sys", "ecm register"),),
    )
