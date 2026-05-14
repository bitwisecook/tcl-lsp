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
            "pem_global_settings_analytics",
            module="pem",
            object_types=("global-settings analytics",),
        ),
        header_types=(("pem", "global-settings analytics"),),
        properties=(
            BigipPropertySpec(name="logging", value_type="unknown"),
            BigipPropertySpec(name="hsl", value_type="unknown", in_sections=("logging",)),
            BigipPropertySpec(
                name="endpoint-id",
                value_type="unknown",
                in_sections=("logging", "hsl"),
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="subscriber-aware",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
