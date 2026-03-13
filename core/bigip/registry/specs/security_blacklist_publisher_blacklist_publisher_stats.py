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
            "security_blacklist_publisher_blacklist_publisher_stats",
            module="security",
            object_types=("blacklist-publisher blacklist-publisher-stats",),
        ),
        header_types=(("security", "blacklist-publisher blacklist-publisher-stats"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
