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
            "apm_report_custom_report_field",
            module="apm",
            object_types=("report custom-report-field",),
        ),
        header_types=(("apm", "report custom-report-field"),),
        properties=(
            BigipPropertySpec(name="alias", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string"),
            BigipPropertySpec(name="field-position", value_type="integer"),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="report-name", value_type="string"),
            BigipPropertySpec(
                name="sort-direction",
                value_type="enum",
                enum_values=("asc", "desc", "unsorted"),
            ),
            BigipPropertySpec(name="sort-order", value_type="integer"),
        ),
    )
