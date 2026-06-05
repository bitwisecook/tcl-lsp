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
            "apm_apm_avr_config",
            module="apm",
            object_types=("apm-avr-config",),
        ),
        header_types=(("apm", "apm-avr-config"),),
        properties=(
            BigipPropertySpec(
                name="avr-collect-data",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="avr-sampling",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
        ),
    )
