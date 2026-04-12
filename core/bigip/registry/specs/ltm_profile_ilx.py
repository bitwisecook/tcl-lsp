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
            "ltm_profile_ilx",
            module="ltm",
            object_types=("profile ilx",),
        ),
        header_types=(("ltm", "profile ilx"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_ilx",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="plugin", value_type="string"),
        ),
    )
