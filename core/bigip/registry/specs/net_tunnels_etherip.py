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
            "net_tunnels_etherip",
            module="net",
            object_types=("tunnels etherip",),
        ),
        header_types=(("net", "tunnels etherip"),),
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
                references=("net_tunnels_etherip",),
                default="etherip",
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
