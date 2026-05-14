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
            "ltm_auth_ssl_cc_ldap",
            module="ltm",
            object_types=("auth ssl-cc-ldap",),
        ),
        header_types=(("ltm", "auth ssl-cc-ldap"),),
        properties=(
            BigipPropertySpec(name="admin-dn", value_type="reference", allow_none=True),
            BigipPropertySpec(name="admin-password", value_type="unknown"),
            BigipPropertySpec(name="cache-size", value_type="integer"),
            BigipPropertySpec(name="cache-timeout", value_type="integer"),
            BigipPropertySpec(name="certmap-base", value_type="unknown"),
            BigipPropertySpec(name="certmap-key", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="certmap-user-serial",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="group-base", value_type="unknown"),
            BigipPropertySpec(name="group-key", value_type="reference", allow_none=True),
            BigipPropertySpec(name="group-member-key", value_type="reference", allow_none=True),
            BigipPropertySpec(name="role-key", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="search-type",
                value_type="enum",
                enum_values=("cert", "certmap", "user"),
            ),
            BigipPropertySpec(name="secure", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="servers", value_type="unknown"),
            BigipPropertySpec(name="user-base", value_type="unknown"),
            BigipPropertySpec(name="user-class", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="user-key", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="valid-groups", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="valid-roles", value_type="unknown", allow_none=True),
        ),
    )
