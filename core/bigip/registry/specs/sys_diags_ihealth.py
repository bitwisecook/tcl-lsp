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
            "sys_diags_ihealth",
            module="sys",
            object_types=("diags ihealth",),
        ),
        header_types=(("sys", "diags ihealth"),),
    )
