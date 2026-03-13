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
            "ltm_profile_qoe",
            module="ltm",
            object_types=("profile qoe",),
        ),
        header_types=(("ltm", "profile qoe"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_qoe",)
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="video", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
