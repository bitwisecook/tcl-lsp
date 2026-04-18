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
            "ltm_classification_signature_definition",
            module="ltm",
            object_types=("classification signature-definition",),
        ),
        header_types=(("ltm", "classification signature-definition"),),
        properties=(
            BigipPropertySpec(
                name="last-attempt-automatic-mode",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="last-attempt-datetime", value_type="string"),
            BigipPropertySpec(name="last-attempt-user", value_type="string"),
            BigipPropertySpec(
                name="last-update-automatic-mode",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="last-update-datetime", value_type="string"),
            BigipPropertySpec(name="last-update-user", value_type="string"),
            BigipPropertySpec(name="message", value_type="string"),
            BigipPropertySpec(
                name="progress-status",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "success", "failure", "in-progress"),
            ),
        ),
    )
