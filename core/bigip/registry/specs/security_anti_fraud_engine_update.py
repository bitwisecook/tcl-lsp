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
            "security_anti_fraud_engine_update",
            module="security",
            object_types=("anti-fraud engine-update",),
        ),
        header_types=(("security", "anti-fraud engine-update"),),
        properties=(
            BigipPropertySpec(name="load", value_type="string"),
            BigipPropertySpec(name="file", value_type="string"),
        ),
    )
