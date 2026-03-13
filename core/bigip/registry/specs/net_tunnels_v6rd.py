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
            "net_tunnels_v6rd",
            module="net",
            object_types=("tunnels v6rd",),
        ),
        header_types=(("net", "tunnels v6rd"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="v6rdprefix", value_type="string"),
            BigipPropertySpec(name="v6rdprefixlen", value_type="integer"),
            BigipPropertySpec(name="ipv4prefix", value_type="string"),
            BigipPropertySpec(name="ipv4prefixlen", value_type="integer"),
        ),
    )
