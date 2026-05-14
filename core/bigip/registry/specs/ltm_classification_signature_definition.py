from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_classification_signature_definition",
            module="ltm",
            object_types=("classification signature-definition",),
        ),
        header_types=(("ltm", "classification signature-definition"),),
        properties=(),
    )
