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
            "security_anti_fraud_signatures_update",
            module="security",
            object_types=("anti-fraud signatures-update",),
        ),
        header_types=(("security", "anti-fraud signatures-update"),),
        properties=(
            BigipPropertySpec(
                name="update-automatically", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="load", value_type="string"),
            BigipPropertySpec(name="file", value_type="string"),
        ),
    )
