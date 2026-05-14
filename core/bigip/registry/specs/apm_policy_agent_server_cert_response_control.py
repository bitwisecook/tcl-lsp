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
            "apm_policy_agent_server_cert_response_control",
            module="apm",
            object_types=("policy agent server-cert-response-control",),
        ),
        header_types=(("apm", "policy agent server-cert-response-control"),),
        properties=(
            BigipPropertySpec(
                name="action",
                value_type="integer",
                default="ignore which specifies that the system ignores untrusted/expired certificate and may allow the connection",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
        ),
    )
