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
            "net_lldp_neighbors",
            module="net",
            object_types=("lldp-neighbors",),
        ),
        header_types=(("net", "lldp-neighbors"),),
    )
