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
            "pem_global_settings_policy",
            module="pem",
            object_types=("global-settings policy",),
        ),
        header_types=(("pem", "global-settings policy"),),
        properties=(
            BigipPropertySpec(
                name="mandatory-action-list",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="report-flow", value_type="unknown"),
            BigipPropertySpec(
                name="single-rule-match-mode",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
