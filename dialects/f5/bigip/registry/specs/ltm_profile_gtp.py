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
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_gtp",),
                default="gtp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ingress-max", value_type="integer", default="0"),
        ),
    )
