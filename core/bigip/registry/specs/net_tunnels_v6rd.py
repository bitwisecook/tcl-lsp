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
                references=("net_tunnels_v6rd",),
                default="v6rd",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ipv4prefix",
                value_type="string",
                shape_kind="ip-address",
                default="0",
            ),
            BigipPropertySpec(name="ipv4prefixlen", value_type="integer", required=True),
            BigipPropertySpec(name="v6rdprefix", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="v6rdprefixlen", value_type="integer", default="56"),
        ),
    )
