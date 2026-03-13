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
            "security_protocol_inspection_updates",
            module="security",
            object_types=("protocol-inspection updates",),
        ),
        header_types=(("security", "protocol-inspection updates"),),
        properties=(
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(name="file", value_type="string"),
        ),
    )
