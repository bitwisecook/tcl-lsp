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
            "net_tunnels_ppp",
            module="net",
            object_types=("tunnels ppp",),
        ),
        header_types=(("net", "tunnels ppp"),),
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
                references=("net_tunnels_ppp",),
                default="ppp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="lcp-echo-failure", value_type="integer", default="4"),
            BigipPropertySpec(name="lcp-echo-interval", value_type="integer", default="30"),
            BigipPropertySpec(
                name="vj",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
