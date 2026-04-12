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
            "net_tunnels_endpoint",
            module="net",
            object_types=("tunnels endpoint",),
        ),
        header_types=(("net", "tunnels endpoint"),),
        properties=(
            BigipPropertySpec(
                name="remote-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
