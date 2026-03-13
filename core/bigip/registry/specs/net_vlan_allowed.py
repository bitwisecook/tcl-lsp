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
            "net_vlan_allowed",
            module="net",
            object_types=("vlan-allowed",),
        ),
        header_types=(("net", "vlan-allowed"),),
    )
