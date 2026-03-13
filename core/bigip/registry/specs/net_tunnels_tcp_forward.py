from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "net_tunnels_tcp_forward",
            module="net",
            object_types=("tunnels tcp-forward",),
        ),
        header_types=(("net", "tunnels tcp-forward"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
