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
            "net_lacp_globals",
            module="net",
            object_types=("lacp-globals",),
        ),
        header_types=(("net", "lacp-globals"),),
    )
