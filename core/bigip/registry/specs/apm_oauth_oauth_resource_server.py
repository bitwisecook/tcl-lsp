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
            "apm_oauth_oauth_resource_server",
            module="apm",
            object_types=("oauth oauth-resource-server",),
        ),
        header_types=(("apm", "oauth oauth-resource-server"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auth-type",
                value_type="enum",
                allow_none=True,
                enum_values=("certificate", "none", "secret"),
                default="certificate and other possible values are none and secret",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="regenerate-resource-server-secret", value_type="unknown"),
            BigipPropertySpec(name="resource-server-cert-dn", value_type="string", allow_none=True),
        ),
    )
