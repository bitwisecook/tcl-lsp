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
            "apm_policy_agent_ending_deny",
            module="apm",
            object_types=("policy agent ending-deny",),
        ),
        header_types=(("apm", "policy agent ending-deny"),),
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
                required=True,
                references=("apm_policy_customization_group",),
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
