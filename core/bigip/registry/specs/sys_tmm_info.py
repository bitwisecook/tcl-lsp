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
            "sys_tmm_info",
            module="sys",
            object_types=("tmm-info",),
        ),
        header_types=(("sys", "tmm-info"),),
    )
