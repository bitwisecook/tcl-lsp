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
            "ltm_classification_signature_update_schedule",
            module="ltm",
            object_types=("classification signature-update-schedule",),
        ),
        header_types=(("ltm", "classification signature-update-schedule"),),
        properties=(
            BigipPropertySpec(
                name="auto-update-interval",
                value_type="enum",
                enum_values=("daily", "weekly", "monthly"),
            ),
        ),
    )
