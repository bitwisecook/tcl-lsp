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
            "ltm_auth_ldap",
            module="ltm",
            object_types=("auth ldap",),
        ),
        header_types=(("ltm", "auth ldap"),),
        properties=(
            BigipPropertySpec(
                name="bind-dn",
                value_type="unknown",
                required=True,
                allow_none=True,
                default="none",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="bind-pw",
                value_type="string",
                required=True,
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="bind-timeout", value_type="integer", default="30 seconds"),
            BigipPropertySpec(
                name="check-host-attr",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="filter", value_type="string", allow_none=True, default="none"),
            BigipPropertySpec(
                name="group-dn",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="group-member-attr", value_type="string", allow_none=True),
            BigipPropertySpec(name="idle-timeout", value_type="integer", default="3600 seconds"),
            BigipPropertySpec(
                name="ignore-auth-info-unavail",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="ignore-unknown-user",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="login-attribute",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="port",
                value_type="reference",
                references=(
                    "net_port_list",
                    "net_port_mirror",
                    "security_firewall_port_list",
                    "security_firewall_port_misuse_policy",
                    "sys_log_config_destination_management_port",
                ),
                default="ldap",
            ),
            BigipPropertySpec(
                name="scope",
                value_type="enum",
                enum_values=("base", "one", "sub"),
                default="sub",
            ),
            BigipPropertySpec(
                name="search-base-dn",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="search-timeout", value_type="unknown", default="30 seconds"),
            BigipPropertySpec(name="servers", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="ssl",
                value_type="enum",
                required=True,
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="ssl-ca-cert-file",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ssl-check-peer",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="ssl-ciphers",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ssl-client-cert",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="ssl-client-key",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="user-template",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="version", value_type="unknown", default="3"),
            BigipPropertySpec(
                name="warnings",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
