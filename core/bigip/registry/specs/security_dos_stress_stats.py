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
            "security_dos_stress_stats",
            module="security",
            object_types=("dos stress-stats",),
        ),
        header_types=(("security", "dos stress-stats"),),
        properties=(
            BigipPropertySpec(name="context-name", value_type="string"),
            BigipPropertySpec(name="stress", value_type="enum", enum_values=("0-100", "auto")),
        ),
    )
