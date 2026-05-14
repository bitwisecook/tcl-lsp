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
            "apm_policy_agent_tacacsplus",
            module="apm",
            object_types=("policy agent tacacsplus",),
        ),
        header_types=(("apm", "policy agent tacacsplus"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-logon-attempt", value_type="integer"),
            BigipPropertySpec(name="server", value_type="string", allow_none=True),
        ),
    )
