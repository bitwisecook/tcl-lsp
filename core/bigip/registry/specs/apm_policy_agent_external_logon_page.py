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
            "apm_policy_agent_external_logon_page",
            module="apm",
            object_types=("policy agent external-logon-page",),
        ),
        header_types=(("apm", "policy agent external-logon-page"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="split-username",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="uri", value_type="string", allow_none=True),
        ),
    )
