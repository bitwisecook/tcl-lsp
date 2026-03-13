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
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="fqdns", value_type="enum", enum_values=("add", "delete", "replace-all-with")
            ),
            BigipPropertySpec(
                name="geo",
                value_type="enum",
                enum_values=("add", "default", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(name="addresses", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("addresses",)),
        ),
    )
