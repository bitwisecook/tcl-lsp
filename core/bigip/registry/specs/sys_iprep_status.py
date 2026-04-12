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
            "sys_iprep_status",
            module="sys",
            object_types=("iprep-status",),
        ),
        header_types=(("sys", "iprep-status"),),
    )
