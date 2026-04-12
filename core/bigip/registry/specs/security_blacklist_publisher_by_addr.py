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
            "security_blacklist_publisher_by_addr",
            module="security",
            object_types=("blacklist-publisher by-addr",),
        ),
        header_types=(("security", "blacklist-publisher by-addr"),),
    )
