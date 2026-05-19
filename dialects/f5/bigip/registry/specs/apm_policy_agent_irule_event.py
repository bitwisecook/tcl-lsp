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
            "apm_policy_agent_irule_event",
            module="apm",
            object_types=("policy agent irule-event",),
        ),
        header_types=(("apm", "policy agent irule-event"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="expect-data", value_type="unknown"),
            BigipPropertySpec(name="id", value_type="string", allow_none=True, default="none"),
        ),
    )
