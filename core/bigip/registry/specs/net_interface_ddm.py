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
            "net_interface_ddm",
            module="net",
            object_types=("interface-ddm",),
        ),
        header_types=(("net", "interface-ddm"),),
    )
