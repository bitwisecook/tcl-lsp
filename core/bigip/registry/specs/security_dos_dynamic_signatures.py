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
            "security_dos_dynamic_signatures",
            module="security",
            object_types=("dos dynamic-signatures",),
        ),
        header_types=(("security", "dos dynamic-signatures"),),
        properties=(
            BigipPropertySpec(name="context-name", value_type="reference"),
            BigipPropertySpec(name="detection-threshold", value_type="integer"),
            BigipPropertySpec(name="dynamic-vectors", value_type="unknown"),
            BigipPropertySpec(
                name="enforce",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="mitigation-threshold", value_type="integer"),
            BigipPropertySpec(
                name="status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
