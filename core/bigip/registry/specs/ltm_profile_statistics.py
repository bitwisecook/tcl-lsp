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
            "ltm_profile_statistics",
            module="ltm",
            object_types=("profile statistics",),
        ),
        header_types=(("ltm", "profile statistics"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_statistics",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="field1", value_type="string"),
            BigipPropertySpec(name="field2", value_type="string"),
            BigipPropertySpec(name="field3", value_type="string"),
            BigipPropertySpec(name="field4", value_type="string"),
            BigipPropertySpec(name="field5", value_type="string"),
            BigipPropertySpec(name="field6", value_type="string"),
            BigipPropertySpec(name="field7", value_type="string"),
            BigipPropertySpec(name="field8", value_type="string"),
            BigipPropertySpec(name="field9", value_type="string"),
            BigipPropertySpec(name="field10", value_type="string"),
            BigipPropertySpec(name="field11", value_type="string"),
            BigipPropertySpec(name="field12", value_type="string"),
            BigipPropertySpec(name="field13", value_type="string"),
            BigipPropertySpec(name="field14", value_type="string"),
            BigipPropertySpec(name="field15", value_type="string"),
            BigipPropertySpec(name="field16", value_type="string"),
            BigipPropertySpec(name="field17", value_type="string"),
            BigipPropertySpec(name="field18", value_type="string"),
            BigipPropertySpec(name="field19", value_type="string"),
            BigipPropertySpec(name="field20", value_type="string"),
            BigipPropertySpec(name="field21", value_type="string"),
            BigipPropertySpec(name="field22", value_type="string"),
            BigipPropertySpec(name="field23", value_type="string"),
            BigipPropertySpec(name="field24", value_type="string"),
            BigipPropertySpec(name="field25", value_type="string"),
            BigipPropertySpec(name="field26", value_type="string"),
            BigipPropertySpec(name="field27", value_type="string"),
            BigipPropertySpec(name="field28", value_type="string"),
            BigipPropertySpec(name="field29", value_type="string"),
            BigipPropertySpec(name="field30", value_type="string"),
            BigipPropertySpec(name="field31", value_type="string"),
            BigipPropertySpec(name="field32", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
