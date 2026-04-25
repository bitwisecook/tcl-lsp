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
            "security_firewall_ipi_category_info",
            module="security",
            object_types=("firewall ipi-category-info",),
        ),
        header_types=(("security", "firewall ipi-category-info"),),
    )
