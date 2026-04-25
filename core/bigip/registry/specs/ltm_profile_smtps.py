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
            "ltm_profile_smtps",
            module="ltm",
            object_types=("profile smtps",),
        ),
        header_types=(("ltm", "profile smtps"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_smtps",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="activation-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "allow", "require"),
            ),
        ),
    )
