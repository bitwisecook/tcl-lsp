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
            "net_tunnels_fec_stat",
            module="net",
            object_types=("tunnels fec-stat",),
        ),
        header_types=(("net", "tunnels fec-stat"),),
    )
