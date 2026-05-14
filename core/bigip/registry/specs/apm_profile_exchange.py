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
            "apm_profile_exchange",
            module="apm",
            object_types=("profile exchange",),
        ),
        header_types=(("apm", "profile exchange"),),
        properties=(
            BigipPropertySpec(
                name="active-sync-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(name="active-sync-sso-config", value_type="string", allow_none=True),
            BigipPropertySpec(name="active-sync-url", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="auto-discover-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="auto-discover-sso-config",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="auto-discover-url", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="ntlm-auth-name", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="offline-address-book-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="offline-address-book-sso-config",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="offline-address-book-url",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="rpc-over-http-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(
                name="rpc-over-http-sso-config",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="rpc-over-http-url", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="user-agent-pattern-for-utf8",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="web-service-auth-type",
                value_type="enum",
                enum_values=("basic", "basic-ntlm", "ntlm"),
            ),
            BigipPropertySpec(name="web-service-sso-config", value_type="string", allow_none=True),
            BigipPropertySpec(name="web-service-url", value_type="string", allow_none=True),
        ),
    )
