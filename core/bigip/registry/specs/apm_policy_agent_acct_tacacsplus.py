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
            "apm_policy_agent_acct_tacacsplus",
            module="apm",
            object_types=("policy agent acct-tacacsplus",),
        ),
        header_types=(("apm", "policy agent acct-tacacsplus"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="server", value_type="string", allow_none=True),
        ),
    )
