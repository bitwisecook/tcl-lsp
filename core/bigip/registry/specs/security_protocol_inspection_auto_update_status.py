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
            "security_protocol_inspection_auto_update_status",
            module="security",
            object_types=("protocol-inspection auto-update status",),
        ),
        header_types=(("security", "protocol-inspection auto-update status"),),
        properties=(
            BigipPropertySpec(
                name="last-updated-time", value_type="reference", references=("auth_partition",)
            ),
            BigipPropertySpec(
                name="message", value_type="string", in_sections=("last-updated-time",)
            ),
        ),
    )
