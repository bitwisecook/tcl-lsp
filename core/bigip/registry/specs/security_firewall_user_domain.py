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
            "security_firewall_user_domain",
            module="security",
            object_types=("firewall user-domain",),
        ),
        header_types=(("security", "firewall user-domain"),),
        properties=(
            BigipPropertySpec(name="domain", value_type="string"),
            BigipPropertySpec(
                name="ifmap-service",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
