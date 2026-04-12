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
            "net_sfc_hop",
            module="net",
            object_types=("sfc hop",),
        ),
        header_types=(("net", "sfc hop"),),
    )
