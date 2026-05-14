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
            "apm_aaa_oauth_provider",
            module="apm",
            object_types=("aaa oauth-provider",),
        ),
        header_types=(("apm", "aaa oauth-provider"),),
        properties=(
            BigipPropertySpec(name="allow-self-signed-jwk-cert", value_type="unknown"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="authentication-uri", value_type="string", allow_none=True),
            BigipPropertySpec(name="auto-jwt-config-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="ignore-expired-cert", value_type="unknown"),
            BigipPropertySpec(name="last-discovery-time", value_type="unknown"),
            BigipPropertySpec(name="manual-jwt-config-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="max-json-nesting-layers", value_type="integer"),
            BigipPropertySpec(name="max-response-size", value_type="integer"),
            BigipPropertySpec(name="openid-cfg-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="save-json-payload",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="token-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="token-validation-scope-uri",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("custom", "f5", "facebook", "google", "ping"),
            ),
            BigipPropertySpec(name="use-auto-jwt-config", value_type="unknown"),
            BigipPropertySpec(name="userinfo-request-uri", value_type="string", allow_none=True),
        ),
    )
