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
            "wom_diagnose_conn",
            module="wom",
            object_types=("diagnose-conn",),
        ),
        header_types=(("wom", "diagnose-conn"),),
        properties=(),
    )
