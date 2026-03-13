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
            "ltm_monitor_none",
            module="ltm",
            object_types=("monitor none",),
        ),
        header_types=(("ltm", "monitor none"),),
    )
