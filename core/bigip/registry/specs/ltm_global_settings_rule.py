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
            "ltm_global_settings_rule",
            module="ltm",
            object_types=("global-settings rule",),
        ),
        header_types=(("ltm", "global-settings rule"),),
        properties=(BigipPropertySpec(name="rule-aborted-log-ratio", value_type="integer"),),
    )
