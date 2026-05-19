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
            "apm_oauth_oauth_client_app",
            module="apm",
            object_types=("oauth oauth-client-app",),
        ),
        header_types=(("apm", "oauth oauth-client-app"),),
        properties=(
            BigipPropertySpec(
                name="access-token-lifetime",
                value_type="integer",
                default="5 minutes",
            ),
            BigipPropertySpec(
                name="allow-plain-code-challenge",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="app-description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="app-name", value_type="string"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="audience",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="auth-code-lifetime", value_type="integer", default="5 minutes"),
            BigipPropertySpec(
                name="auth-type",
                value_type="enum",
                allow_none=True,
                enum_values=("certificate", "none", "secret"),
                default="secret and other possible values are none and certificate",
            ),
            BigipPropertySpec(name="client-cert-dn", value_type="string", allow_none=True),
            BigipPropertySpec(name="contact", value_type="string", allow_none=True),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="generate-jwt-refresh-token",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="generate-refresh-token",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="grant-code",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="grant-password",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="grant-token",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="id-token-claims",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="id-token-lifetime", value_type="integer", default="5 minutes"),
            BigipPropertySpec(
                name="jwt-access-token-claims",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="jwt-access-token-lifetime",
                value_type="integer",
                default="5 minutes",
            ),
            BigipPropertySpec(
                name="jwt-refresh-token-lifetime",
                value_type="integer",
                default="60 minutes",
            ),
            BigipPropertySpec(name="logo-url", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="openid-connect",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="redirect-uris",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="refresh-token-lifetime",
                value_type="integer",
                default="480 minutes",
            ),
            BigipPropertySpec(name="refresh-token-usage-limit", value_type="integer", default="64"),
            BigipPropertySpec(name="regenerate-client-secret", value_type="unknown"),
            BigipPropertySpec(
                name="require-pkce",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="reuse-access-token",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="reuse-refresh-token",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="scopes",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="use-profile-token-mgmt-settings",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="userinfo-claims",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="website-url", value_type="string", allow_none=True),
        ),
    )
