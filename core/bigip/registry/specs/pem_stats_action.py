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
            "pem_stats_action",
            module="pem",
            object_types=("stats action",),
        ),
        header_types=(("pem", "stats action"),),
        properties=(),
    )
