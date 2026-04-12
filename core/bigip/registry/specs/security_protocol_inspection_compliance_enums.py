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
            "security_protocol_inspection_compliance_enums",
            module="security",
            object_types=("protocol-inspection compliance-enums",),
        ),
        header_types=(("security", "protocol-inspection compliance-enums"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="insp-id", value_type="integer"),
            BigipPropertySpec(name="value", value_type="string"),
        ),
    )
