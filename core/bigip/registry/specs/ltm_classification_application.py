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
            "ltm_classification_application",
            module="ltm",
            object_types=("classification application",),
        ),
        header_types=(("ltm", "classification application"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="application-id", value_type="integer"),
            BigipPropertySpec(name="category", value_type="string"),
        ),
    )
