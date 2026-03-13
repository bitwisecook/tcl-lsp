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
            "ltm_classification_auto_update_settings",
            module="ltm",
            object_types=("classification auto-update settings",),
        ),
        header_types=(("ltm", "classification auto-update settings"),),
        properties=(
            BigipPropertySpec(
                name="auto-update-interval",
                value_type="enum",
                enum_values=("daily", "monthly", "weekly"),
            ),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("yes", "no")),
        ),
    )
