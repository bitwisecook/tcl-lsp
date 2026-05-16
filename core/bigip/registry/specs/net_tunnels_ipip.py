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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("net_tunnels_ipip",),
                default="ipip",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ds-lite", value_type="unknown", default="disabled"),
            BigipPropertySpec(
                name="proto",
                value_type="enum",
                enum_values=("IPv4", "IPv6"),
                shape_kind="ip-address",
                default="IPv4",
            ),
        ),
    )
