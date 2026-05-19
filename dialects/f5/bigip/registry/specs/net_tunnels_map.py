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
            "net_tunnels_map",
            module="net",
            object_types=("tunnels map",),
        ),
        header_types=(("net", "tunnels map"),),
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
                references=("net_tunnels_map",),
                default="map",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ea-bits-length",
                value_type="integer",
                default="32 (IPv4 prefix 24 bits + PSID 8 bits)",
            ),
            BigipPropertySpec(name="ip4-prefix", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="ip6-prefix", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="port-offset", value_type="integer", default="6"),
        ),
    )
