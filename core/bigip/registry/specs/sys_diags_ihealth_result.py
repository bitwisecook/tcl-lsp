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
            "sys_diags_ihealth_result",
            module="sys",
            object_types=("diags ihealth-result",),
        ),
        header_types=(("sys", "diags ihealth-result"),),
    )
