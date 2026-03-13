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
            "gtm_global_settings_metrics_exclusions",
            module="gtm",
            object_types=("global-settings metrics-exclusions",),
        ),
        header_types=(("gtm", "global-settings metrics-exclusions"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
        ),
    )
