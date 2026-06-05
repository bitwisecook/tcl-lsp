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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("net_tunnels_tcp_forward",),
                default="tcp-forward",
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
