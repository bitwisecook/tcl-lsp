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
            "net_sfc_sf",
            module="net",
            object_types=("sfc sf",),
        ),
        header_types=(("net", "sfc sf"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="egress-interface", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ingress-interface", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="ip-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="nsh-aware", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="pool-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="virtual-name", value_type="boolean", allow_none=True),
        ),
    )
