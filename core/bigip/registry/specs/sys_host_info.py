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
            "sys_host_info",
            module="sys",
            object_types=("host-info",),
        ),
        header_types=(("sys", "host-info"),),
    )
