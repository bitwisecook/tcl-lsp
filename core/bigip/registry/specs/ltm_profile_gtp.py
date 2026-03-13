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
            "ltm_profile_gtp",
            module="ltm",
            object_types=("profile gtp",),
        ),
        header_types=(("ltm", "profile gtp"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_gtp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ingress-max", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
