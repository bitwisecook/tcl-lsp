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
            "pem_global_settings_gx",
            module="pem",
            object_types=("global-settings gx",),
        ),
        header_types=(("pem", "global-settings gx"),),
        properties=(
            BigipPropertySpec(
                name="gx-immediate-reporting",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="gx-usage-record-merge",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
