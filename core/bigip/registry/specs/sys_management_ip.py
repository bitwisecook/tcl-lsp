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
            "sys_management_ip",
            module="sys",
            object_types=("management-ip",),
        ),
        header_types=(("sys", "management-ip"),),
    )
