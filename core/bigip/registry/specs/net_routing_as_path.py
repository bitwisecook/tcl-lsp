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
            "net_routing_as_path",
            module="net",
            object_types=("routing as-path",),
        ),
        header_types=(("net", "routing as-path"),),
        properties=(),
    )
