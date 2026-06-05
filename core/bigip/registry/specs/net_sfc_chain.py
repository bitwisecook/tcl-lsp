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
            "net_sfc_chain",
            module="net",
            object_types=("sfc chain",),
        ),
        header_types=(("net", "sfc chain"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="hopkey",
                value_type="enum",
                enum_values=("interface", "service-index"),
            ),
            BigipPropertySpec(name="hops", value_type="unknown"),
            BigipPropertySpec(name="path-id", value_type="integer"),
            BigipPropertySpec(name="service-index", value_type="integer"),
            BigipPropertySpec(name="source-interface", value_type="string", allow_none=True),
        ),
    )
