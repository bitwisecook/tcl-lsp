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
            "ltm_classification_updates",
            module="ltm",
            object_types=("classification updates",),
        ),
        header_types=(("ltm", "classification updates"),),
        properties=(
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(name="file", value_type="string"),
            BigipPropertySpec(name="load", value_type="string"),
        ),
    )
