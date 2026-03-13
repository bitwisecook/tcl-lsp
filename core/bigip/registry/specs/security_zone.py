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
            "security_zone",
            module="security",
            object_types=("zone",),
        ),
        header_types=(("security", "zone"),),
        properties=(
            BigipPropertySpec(name="copy-from", value_type="string"),
            BigipPropertySpec(
                name="vlans",
                value_type="reference",
                enum_values=("add", "delete", "modify", "replace-all-with"),
                references=("net_vlan",),
            ),
        ),
    )
