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
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="app-service", value_type="reference"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="address-lists",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
