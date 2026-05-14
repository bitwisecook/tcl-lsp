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
            "apm_policy_agent_endpoint_windows_group_policy",
            module="apm",
            object_types=("policy agent endpoint-windows-group-policy",),
        ),
        header_types=(("apm", "policy agent endpoint-windows-group-policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="policy-file", value_type="unknown"),
        ),
    )
