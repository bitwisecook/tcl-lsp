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
            "ltm_persistence_global_settings",
            module="ltm",
            object_types=("persistence global-settings",),
        ),
        header_types=(("ltm", "persistence global-settings"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dest-addr-limit-mode", value_type="enum", enum_values=("timeout", "maxcount")
            ),
            BigipPropertySpec(name="dest-addr-max", value_type="integer"),
            BigipPropertySpec(name="proxy-group", value_type="string"),
        ),
    )
