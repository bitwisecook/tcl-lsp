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
            "apm_policy_agent_endpoint_check_machine_cert",
            module="apm",
            object_types=("policy agent endpoint-check-machine-cert",),
        ),
        header_types=(("apm", "policy agent endpoint-check-machine-cert"),),
        properties=(
            BigipPropertySpec(
                name="allow-elevation",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="ca-profile-name", value_type="unknown"),
            BigipPropertySpec(name="issuer", value_type="unknown"),
            BigipPropertySpec(
                name="save-cert",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="serial-number", value_type="integer"),
            BigipPropertySpec(
                name="store-location",
                value_type="enum",
                enum_values=("machine", "user"),
            ),
            BigipPropertySpec(name="store-name", value_type="unknown"),
            BigipPropertySpec(name="subject-alt-name", value_type="unknown"),
            BigipPropertySpec(name="subject-match-fqdn", value_type="unknown"),
        ),
    )
