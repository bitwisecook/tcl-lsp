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
            "security_protocol_inspection_profile_status",
            module="security",
            object_types=("protocol-inspection profile-status",),
        ),
        header_types=(("security", "protocol-inspection profile-status"),),
    )
