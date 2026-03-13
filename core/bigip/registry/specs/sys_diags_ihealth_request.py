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
            "sys_diags_ihealth_request",
            module="sys",
            object_types=("diags ihealth-request",),
        ),
        header_types=(("sys", "diags ihealth-request"),),
    )
