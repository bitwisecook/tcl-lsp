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
            "apm_policy_agent_decision_box",
            module="apm",
            object_types=("policy agent decision-box",),
        ),
        header_types=(("apm", "policy agent decision-box"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="reference",
                references=("apm_policy_customization_group",),
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
