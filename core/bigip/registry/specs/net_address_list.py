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
            "net_address_list",
            module="net",
            object_types=("address-list",),
        ),
        header_types=(("net", "address-list"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="net", value_type="string"),
            BigipPropertySpec(name="addresses", value_type="string", in_sections=("net",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("addresses",)),
        ),
    )
