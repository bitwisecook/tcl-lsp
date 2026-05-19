from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "wom_deduplication",
            module="wom",
            object_types=("deduplication",),
        ),
        header_types=(("wom", "deduplication"),),
        properties=(
            BigipPropertySpec(name="codec", value_type="enum", enum_values=("sdd-v2", "sdd-v3")),
            BigipPropertySpec(name="max-endpoint-count", value_type="integer"),
        ),
    )
