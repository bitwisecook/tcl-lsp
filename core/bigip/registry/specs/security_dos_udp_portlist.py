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
            "security_dos_udp_portlist",
            module="security",
            object_types=("dos udp-portlist",),
        ),
        header_types=(("security", "dos udp-portlist"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="entries",
                value_type="list",
                list_operators=frozenset(("modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="match-direction",
                value_type="enum",
                in_sections=("entries",),
                allow_none=True,
                enum_values=("both", "dst", "none", "src"),
            ),
            BigipPropertySpec(name="port-number", value_type="unknown", in_sections=("entries",)),
            BigipPropertySpec(name="list-type", value_type="unknown"),
        ),
    )
