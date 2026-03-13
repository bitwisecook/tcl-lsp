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
            "sys_software_update_status",
            module="sys",
            object_types=("software update-status",),
        ),
        header_types=(("sys", "software update-status"),),
    )
