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
            "apm_policy_agent_ending_redirect",
            module="apm",
            object_types=("policy agent ending-redirect",),
        ),
        header_types=(("apm", "policy agent ending-redirect"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="close-session",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="url", value_type="unknown"),
        ),
    )
