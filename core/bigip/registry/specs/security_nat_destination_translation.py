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
            "security_nat_destination_translation",
            module="security",
            object_types=("nat destination-translation",),
        ),
        header_types=(("security", "nat destination-translation"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="type", value_type="enum", enum_values=("static-nat", "static-pat")
            ),
        ),
    )
