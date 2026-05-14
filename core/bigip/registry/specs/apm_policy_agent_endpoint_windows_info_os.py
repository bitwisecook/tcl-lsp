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
            "apm_policy_agent_endpoint_windows_info_os",
            module="apm",
            object_types=("policy agent endpoint-windows-info-os",),
        ),
        header_types=(("apm", "policy agent endpoint-windows-info-os"),),
        properties=(BigipPropertySpec(name="app-service", value_type="string", allow_none=True),),
    )
