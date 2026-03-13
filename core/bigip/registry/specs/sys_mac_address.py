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
            "sys_mac_address",
            module="sys",
            object_types=("mac-address",),
        ),
        header_types=(("sys", "mac-address"),),
    )
