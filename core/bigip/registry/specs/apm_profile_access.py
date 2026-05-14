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
            "apm_profile_access",
            module="apm",
            object_types=("profile access",),
        ),
        header_types=(("apm", "profile access"),),
        properties=(
            BigipPropertySpec(
                name="accept-languages",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="access-policy", value_type="string", allow_none=True),
            BigipPropertySpec(name="access-policy-timeout", value_type="integer"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="cache-generation", value_type="integer"),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="default-language", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="string",
                allow_none=True,
                references=("apm_profile_access",),
            ),
            BigipPropertySpec(name="domain-cookie", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="domain-groups",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="domain-mode",
                value_type="enum",
                enum_values=("multi-domain", "single-domain"),
            ),
            BigipPropertySpec(
                name="enforce-policy",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="eps-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="errormap-group", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="framework-installation-group",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="general-ui-group", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="generation-action",
                value_type="enum",
                enum_values=("increment", "noop"),
            ),
            BigipPropertySpec(
                name="httponly-cookie",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="inactivity-timeout", value_type="integer"),
            BigipPropertySpec(
                name="log-settings",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="logout-uri-include",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="logout-uri-timeout", value_type="integer"),
            BigipPropertySpec(
                name="max-concurrent-sessions",
                value_type="integer",
                allow_none=True,
            ),
            BigipPropertySpec(name="max-concurrent-users", value_type="integer", allow_none=True),
            BigipPropertySpec(name="max-failure-delay", value_type="integer"),
            BigipPropertySpec(
                name="max-in-progress-sessions",
                value_type="integer",
                allow_none=True,
            ),
            BigipPropertySpec(name="max-session-timeout", value_type="integer"),
            BigipPropertySpec(name="min-failure-delay", value_type="integer"),
            BigipPropertySpec(name="named-scope", value_type="string", allow_none=True),
            BigipPropertySpec(name="oauth-profile", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="persistent-cookie",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="primary-auth-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="restrict-to-single-client-ip",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="sandboxes",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="scope",
                value_type="enum",
                enum_values=("profile", "public", "virtual-server"),
            ),
            BigipPropertySpec(
                name="secure-cookie",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="sso-name", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "all",
                    "identity-service",
                    "ltm-apm",
                    "oauth-resource-server",
                    "rdg-rap",
                    "ssl-vpn",
                    "sso",
                    "swg-explicit",
                    "swg-transparent",
                    "system-authentication",
                ),
            ),
            BigipPropertySpec(
                name="use-http-503-on-error",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="user-identity-method",
                value_type="enum",
                enum_values=("http",),
            ),
        ),
    )
