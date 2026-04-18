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
            "net_sfc_stats",
            module="net",
            object_types=("sfc-stats",),
        ),
        header_types=(("net", "sfc-stats"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
