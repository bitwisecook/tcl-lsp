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
            "apm_policy_agent_kerberos",
            module="apm",
            object_types=("policy agent kerberos",),
        ),
        header_types=(("apm", "policy agent kerberos"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="max-logon-attempt", value_type="integer", default="3"),
            BigipPropertySpec(name="server", value_type="string", required=True),
        ),
    )
