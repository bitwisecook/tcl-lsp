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
            "pem_global_settings_hsl_flow",
            module="pem",
            object_types=("global-settings hsl-flow",),
        ),
        header_types=(("pem", "global-settings hsl-flow"),),
        properties=(
            BigipPropertySpec(
                name="format-version",
                value_type="unknown",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="report-on-interim",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="report-on-start",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
