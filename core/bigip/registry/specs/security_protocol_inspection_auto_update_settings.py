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
            "security_protocol_inspection_auto_update_settings",
            module="security",
            object_types=("protocol-inspection auto-update settings",),
        ),
        header_types=(("security", "protocol-inspection auto-update settings"),),
        properties=(
            BigipPropertySpec(
                name="auto-update-interval", value_type="reference", references=("auth_partition",)
            ),
            BigipPropertySpec(name="enabled", value_type="string"),
        ),
    )
