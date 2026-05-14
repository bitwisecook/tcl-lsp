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
            "security_firewall_address_list",
            module="security",
            object_types=("firewall address-list",),
        ),
        header_types=(("security", "firewall address-list"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="app-service", value_type="reference"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="fqdns",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="geo",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
