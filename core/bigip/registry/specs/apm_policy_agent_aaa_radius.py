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
            "apm_policy_agent_aaa_radius",
            module="apm",
            object_types=("policy agent aaa-radius",),
        ),
        header_types=(("apm", "policy agent aaa-radius"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-logon-attempt", value_type="unknown"),
            BigipPropertySpec(name="password-source", value_type="string", allow_none=True),
            BigipPropertySpec(name="server", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="show-extended-error", value_type="unknown"),
            BigipPropertySpec(name="username-source", value_type="string", allow_none=True),
        ),
    )
