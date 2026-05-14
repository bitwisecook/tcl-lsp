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
            "security_shared_objects_address_list",
            module="security",
            object_types=("shared-objects address-list",),
        ),
        header_types=(("security", "shared-objects address-list"),),
        properties=(),
    )
