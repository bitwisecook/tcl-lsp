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
            "net_mroute",
            module="net",
            object_types=("mroute",),
        ),
        header_types=(("net", "mroute"),),
        properties=(),
    )
