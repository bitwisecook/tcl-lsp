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
            "apm_policy_agent_aaa_client_cert",
            module="apm",
            object_types=("policy agent aaa-client-cert",),
        ),
        header_types=(("apm", "policy agent aaa-client-cert"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("request", "require")),
        ),
    )
