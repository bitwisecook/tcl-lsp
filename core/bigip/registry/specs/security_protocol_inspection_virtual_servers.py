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
            "security_protocol_inspection_virtual_servers",
            module="security",
            object_types=("protocol-inspection virtual-servers",),
        ),
        header_types=(("security", "protocol-inspection virtual-servers"),),
        properties=(BigipPropertySpec(name="field-fmt", value_type="string"),),
    )
