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
            "wom_remote_route",
            module="wom",
            object_types=("remote-route",),
        ),
        header_types=(("wom", "remote-route"),),
        properties=(),
    )
