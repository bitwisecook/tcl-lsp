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
            "sys_dynad_status",
            module="sys",
            object_types=("dynad status",),
        ),
        header_types=(("sys", "dynad status"),),
    )
