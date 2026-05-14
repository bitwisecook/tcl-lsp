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
            "apm_report",
            module="apm",
            object_types=("report",),
        ),
        header_types=(("apm", "report"),),
        properties=(),
    )
