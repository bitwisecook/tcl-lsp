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
            "security_protocol_inspection_common_config",
            module="security",
            object_types=("protocol-inspection common-config",),
        ),
        header_types=(("security", "protocol-inspection common-config"),),
        properties=(
            BigipPropertySpec(name="compliance", value_type="list", repeated=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="service", value_type="list", repeated=True),
        ),
    )
