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
            "sys_fix_connection",
            module="sys",
            object_types=("fix-connection",),
        ),
        header_types=(("sys", "fix-connection"),),
    )
