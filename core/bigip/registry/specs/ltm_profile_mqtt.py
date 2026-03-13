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
            "ltm_profile_mqtt",
            module="ltm",
            object_types=("profile mqtt",),
        ),
        header_types=(("ltm", "profile mqtt"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
