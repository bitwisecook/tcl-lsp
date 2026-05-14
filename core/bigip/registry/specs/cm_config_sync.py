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
            BigipPropertySpec(name="force-full-load-push", value_type="unknown"),
            BigipPropertySpec(name="from-group", value_type="unknown"),
            BigipPropertySpec(name="recover-sync", value_type="unknown"),
            BigipPropertySpec(name="run", value_type="unknown"),
            BigipPropertySpec(name="to-group", value_type="unknown"),
        ),
    )
