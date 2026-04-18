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
            "net_tunnels_ipip",
            module="net",
            object_types=("tunnels ipip",),
        ),
        header_types=(("net", "tunnels ipip"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="proto", value_type="enum", enum_values=("ipv4", "ipv6")),
            BigipPropertySpec(name="ds-lite", value_type="string"),
        ),
    )
