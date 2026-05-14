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
            "security_firewall_matching_rule",
            module="security",
            object_types=("firewall matching-rule",),
        ),
        header_types=(("security", "firewall matching-rule"),),
        properties=(),
    )
