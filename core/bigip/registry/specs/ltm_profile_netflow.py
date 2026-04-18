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
            "ltm_profile_netflow",
            module="ltm",
            object_types=("profile netflow",),
        ),
        header_types=(("ltm", "profile netflow"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_netflow",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="netflow-version", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
