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
            "security_blacklist_publisher_all_blacklist_publisher",
            module="security",
            object_types=("blacklist-publisher all-blacklist-publisher",),
        ),
        header_types=(("security", "blacklist-publisher all-blacklist-publisher"),),
    )
