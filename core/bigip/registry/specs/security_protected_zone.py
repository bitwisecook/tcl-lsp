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
            "security_protected_zone",
            module="security",
            object_types=("protected zone",),
        ),
        header_types=(("security", "protected zone"),),
        properties=(
            BigipPropertySpec(name="copy-from", value_type="string"),
            BigipPropertySpec(name="fw-zone-name", value_type="string"),
            BigipPropertySpec(name="dos-profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="log-profile",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
        ),
    )
