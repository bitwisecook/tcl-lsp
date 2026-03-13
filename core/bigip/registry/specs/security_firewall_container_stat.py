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
            "security_firewall_container_stat",
            module="security",
            object_types=("firewall container-stat",),
        ),
        header_types=(("security", "firewall container-stat"),),
    )
