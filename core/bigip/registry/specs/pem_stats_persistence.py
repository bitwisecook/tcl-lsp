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
            "pem_stats_persistence",
            module="pem",
            object_types=("stats persistence",),
        ),
        header_types=(("pem", "stats persistence"),),
        properties=(),
    )
