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
            "apm_profile_oauth",
            module="apm",
            object_types=("profile oauth",),
        ),
        header_types=(("apm", "profile oauth"),),
        properties=(
            BigipPropertySpec(
                name="access-token-lifetime",
                value_type="integer",
                default="5 minutes",
            ),
            BigipPropertySpec(
                name="allow-plain-code-challenge",
                value_type="enum",
                enum_values=("disabled", "enalbed"),
                default="enabled",
            ),
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
                name="auth-url",
                value_type="string",
                default="/f5-oauth2/v1/authorize",
            ),
            BigipPropertySpec(
                name="client-apps",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="db-instance", value_type="unknown"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="string",
                allow_none=True,
                references=("apm_profile_oauth",),
                default="oauth",
            ),
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
                name="id-token-claims",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="id-token-lifetime", value_type="integer", default="5 minutes"),
            BigipPropertySpec(name="id-token-primary-key", value_type="unknown"),
            BigipPropertySpec(
                name="ignore-expired-cert",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(name="issuer", value_type="string"),
            BigipPropertySpec(name="jwks-url", value_type="string", default="/f5-oauth2/v1/jwks"),
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
                name="jwt-ec-signature-format",
                value_type="enum",
                enum_values=("binary", "der"),
                default="binary format",
            ),
            BigipPropertySpec(name="jwt-refresh-token-enc-secret", value_type="string"),
            BigipPropertySpec(
                name="jwt-refresh-token-lifetime",
                value_type="integer",
                default="60 minutes",
            ),
            BigipPropertySpec(
                name="jwt-token",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="opaque-token",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(name="openid-cfg-url", value_type="string", default="/f5-oauth2/v1/"),
            BigipPropertySpec(
                name="openid-connect",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="per-user-token-limit", value_type="integer", default="255"),
            BigipPropertySpec(name="primary-key", value_type="unknown"),
            BigipPropertySpec(
                name="refresh-token-lifetime",
                value_type="integer",
                default="480 minutes",
            ),
            BigipPropertySpec(
                name="refresh-token-usage-limit",
                value_type="integer",
                default="0, which represents unlimited number of times",
            ),
            BigipPropertySpec(
                name="require-pkce",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="resource-servers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
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
                name="rotation-keys",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="subject",
                value_type="string",
                allow_none=True,
                default="%{session",
            ),
            BigipPropertySpec(
                name="token-introspection-url",
                value_type="string",
                default="/f5-oauth2/v1/introspect",
            ),
            BigipPropertySpec(
                name="token-issuance-url",
                value_type="string",
                default="/f5-oauth2/v1/token",
            ),
            BigipPropertySpec(
                name="token-revocation-url",
                value_type="string",
                default="/f5-oauth2/v1/revoke",
            ),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="unknown"),
            BigipPropertySpec(
                name="userinfo-claims",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="userinfo-primary-key", value_type="unknown"),
            BigipPropertySpec(
                name="userinfo-url",
                value_type="string",
                default="/f5-oauth2/v1/userinfo",
            ),
        ),
    )
