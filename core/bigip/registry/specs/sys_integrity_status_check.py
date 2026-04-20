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
            "sys_integrity_status_check",
            module="sys",
            object_types=("integrity status-check",),
        ),
        header_types=(("sys", "integrity status-check"),),
    )
