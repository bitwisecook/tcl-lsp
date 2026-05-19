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
            "apm_policy_agent_acct_radius",
            module="apm",
            object_types=("policy agent acct-radius",),
        ),
        header_types=(("apm", "policy agent acct-radius"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="server", value_type="string", required=True, allow_none=True),
            BigipPropertySpec(
                name="username-source",
                value_type="string",
                allow_none=True,
                default="%{session",
            ),
        ),
    )
