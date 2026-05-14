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
            "net_tunnels_geneve",
            module="net",
            object_types=("tunnels geneve",),
        ),
        header_types=(("net", "tunnels geneve"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("net_tunnels_geneve",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="flooding-type",
                value_type="enum",
                allow_none=True,
                enum_values=("multicast", "multipoint", "none"),
            ),
            BigipPropertySpec(name="port", value_type="integer"),
        ),
    )
