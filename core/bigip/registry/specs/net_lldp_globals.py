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
            "net_lldp_globals",
            module="net",
            object_types=("lldp-globals",),
        ),
        header_types=(("net", "lldp-globals"),),
    )
