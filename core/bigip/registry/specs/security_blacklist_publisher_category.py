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
            "security_blacklist_publisher_category",
            module="security",
            object_types=("blacklist-publisher category",),
        ),
        header_types=(("security", "blacklist-publisher category"),),
        properties=(
            BigipPropertySpec(
                name="profile-names",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
            ),
        ),
    )
