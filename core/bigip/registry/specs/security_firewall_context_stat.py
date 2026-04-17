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
            "security_firewall_context_stat",
            module="security",
            object_types=("firewall context-stat",),
        ),
        header_types=(("security", "firewall context-stat"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
