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
            "security_protocol_inspection_service",
            module="security",
            object_types=("protocol-inspection service",),
        ),
        header_types=(("security", "protocol-inspection service"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(name="ports", value_type="string"),
        ),
    )
