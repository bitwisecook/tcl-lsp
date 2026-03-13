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
            "ltm_classification_category",
            module="ltm",
            object_types=("classification category",),
        ),
        header_types=(("ltm", "classification category"),),
        properties=(BigipPropertySpec(name="description", value_type="string"),),
    )
