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
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="hopkey", value_type="enum", enum_values=("service-index", "interface")
            ),
            BigipPropertySpec(name="service-index", value_type="integer"),
            BigipPropertySpec(name="source-interface", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="path-id", value_type="integer"),
        ),
    )
