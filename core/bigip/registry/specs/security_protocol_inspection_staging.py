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
            "security_protocol_inspection_staging",
            module="security",
            object_types=("protocol-inspection staging",),
        ),
        header_types=(("security", "protocol-inspection staging"),),
        properties=(BigipPropertySpec(name="field-fmt", value_type="string"),),
    )
