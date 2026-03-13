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
            "cm_config_sync",
            module="cm",
            object_types=("config-sync",),
        ),
        header_types=(("cm", "config-sync"),),
        properties=(
            BigipPropertySpec(name="from-group", value_type="string"),
            BigipPropertySpec(name="to-group", value_type="string"),
        ),
    )
