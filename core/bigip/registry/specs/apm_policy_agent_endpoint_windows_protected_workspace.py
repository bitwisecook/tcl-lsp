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
            "apm_policy_agent_endpoint_windows_protected_workspace",
            module="apm",
            object_types=("policy agent endpoint-windows-protected-workspace",),
        ),
        header_types=(("apm", "policy agent endpoint-windows-protected-workspace"),),
        properties=(
            BigipPropertySpec(
                name="allow-burn-cid",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="allow-printer-use",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="allow-user-switch",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="allowed-network-shares",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="close-google-desktop-search",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="usb-flash-access",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "ironkey", "none"),
            ),
        ),
    )
