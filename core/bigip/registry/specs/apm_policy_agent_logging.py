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
            "apm_policy_agent_logging",
            module="apm",
            object_types=("policy agent logging",),
        ),
        header_types=(("apm", "policy agent logging"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="log-message",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(name="variables", value_type="string", allow_none=True),
        ),
    )
