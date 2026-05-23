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
            "apm_policy_agent_endpoint_windows_check_file",
            module="apm",
            object_types=("policy agent endpoint-windows-check-file",),
        ),
        header_types=(("apm", "policy agent endpoint-windows-check-file"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="continuous-check",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="files",
                value_type="enum",
                enum_values=("md5", "modified", "operation", "signer", "size", "version"),
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
