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
            "net_route",
            module="net",
            object_types=("route",),
        ),
        header_types=(("net", "route"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="gw",
                value_type="reference",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
                references=("net_self", "net_route", "ltm_virtual_address"),
            ),
            BigipPropertySpec(name="interface", value_type="string"),
            BigipPropertySpec(name="mtu", value_type="integer"),
            BigipPropertySpec(name="network", value_type="string"),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="lookup", value_type="string"),
        ),
    )
