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
            "security_firewall_current_state",
            module="security",
            object_types=("firewall current-state",),
        ),
        header_types=(("security", "firewall current-state"),),
    )
