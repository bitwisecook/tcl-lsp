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
            "apm_policy_agent_session_check",
            module="apm",
            object_types=("policy agent session-check",),
        ),
        header_types=(("apm", "policy agent session-check"),),
        properties=(BigipPropertySpec(name="app-service", value_type="string", allow_none=True),),
    )
