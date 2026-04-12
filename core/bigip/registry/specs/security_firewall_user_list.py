from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "security_firewall_user_list",
            module="security",
            object_types=("firewall user-list",),
        ),
        header_types=(("security", "firewall user-list"),),
    )
