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
            "ltm_profile_html",
            module="ltm",
            object_types=("profile html",),
        ),
        header_types=(("ltm", "profile html"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_html",),
            ),
            BigipPropertySpec(
                name="content-detection", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="content-selection", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="rules", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
