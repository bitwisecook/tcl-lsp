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
            "auth_ldap",
            module="auth",
            object_types=("ldap",),
        ),
        header_types=(("auth", "ldap"),),
        properties=(
            BigipPropertySpec(name="bind-dn", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="bind-pw", value_type="enum", allow_none=True, enum_values=("none", "password")
            ),
            BigipPropertySpec(name="bind-timeout", value_type="integer"),
            BigipPropertySpec(
                name="check-host-attr", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="check-roles-group", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="filter", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="group-dn", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="group-member-attr", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="ignore-auth-info-unavail", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="ignore-unknown-user", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(name="login-attribute", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="referrals", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="scope", value_type="enum", enum_values=("base", "one", "sub")),
            BigipPropertySpec(name="search-base-dn", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="search-timeout", value_type="integer"),
            BigipPropertySpec(
                name="servers", value_type="enum", enum_values=("add", "delete", "replace-all-with")
            ),
            BigipPropertySpec(name="ssl", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="ssl-ca-cert-file", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="ssl-check-peer", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="ssl-ciphers", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ssl-client-cert", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ssl-client-key", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="user-template", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="version", value_type="integer"),
            BigipPropertySpec(
                name="warnings", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
