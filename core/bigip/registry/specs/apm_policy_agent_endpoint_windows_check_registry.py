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
            "apm_policy_agent_endpoint_windows_check_registry",
            module="apm",
            object_types=("policy agent endpoint-windows-check-registry",),
        ),
        header_types=(("apm", "policy agent endpoint-windows-check-registry"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="continuous-check",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="expression", value_type="string", allow_none=True),
        ),
    )
