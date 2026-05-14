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
            "apm_policy_agent_variable_assign",
            module="apm",
            object_types=("policy agent variable-assign",),
        ),
        header_types=(("apm", "policy agent variable-assign"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "citrix-smart-access",
                    "general",
                    "intranet-webtop",
                    "sso-cred-mapping",
                    "virtual-keyboard",
                ),
            ),
            BigipPropertySpec(
                name="variables",
                value_type="list",
                references=("apm_policy_agent_variable_assign",),
            ),
        ),
    )
