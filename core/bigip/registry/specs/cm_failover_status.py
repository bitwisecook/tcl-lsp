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
            "cm_failover_status",
            module="cm",
            object_types=("failover-status",),
        ),
        header_types=(("cm", "failover-status"),),
    )
