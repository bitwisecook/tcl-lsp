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
            "gtm_prober_pool",
            module="gtm",
            object_types=("prober-pool",),
        ),
        header_types=(("gtm", "prober-pool"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="load-balancing-mode",
                value_type="enum",
                enum_values=("global-availability", "round-robin"),
            ),
            BigipPropertySpec(name="members", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
