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
            "security_ip_intelligence_blacklist_category",
            module="security",
            object_types=("ip-intelligence blacklist-category",),
        ),
        header_types=(("security", "ip-intelligence blacklist-category"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="bl-match-direction",
                value_type="enum",
                enum_values=("destination", "source", "source-and-destination"),
            ),
        ),
    )
