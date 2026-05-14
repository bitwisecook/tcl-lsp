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
            "apm_policy_agent_aaa_ldap",
            module="apm",
            object_types=("policy agent aaa-ldap",),
        ),
        header_types=(("apm", "policy agent aaa-ldap"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="attr-name",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete")),
            ),
            BigipPropertySpec(name="filter", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="group-member-scope",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "direct", "none"),
            ),
            BigipPropertySpec(
                name="group-membership-scope",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "direct", "none"),
            ),
            BigipPropertySpec(
                name="ldapmod-attributes",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete")),
            ),
            BigipPropertySpec(name="max-logon-attempt", value_type="integer"),
            BigipPropertySpec(
                name="modify-type",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify")),
            ),
            BigipPropertySpec(name="search-dn", value_type="string", allow_none=True),
            BigipPropertySpec(name="server", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="show-extended-error",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="list",
                list_operators=frozenset(("modify",)),
            ),
            BigipPropertySpec(name="user-dn", value_type="string", allow_none=True),
        ),
    )
