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
            "auth_cert_ldap",
            module="auth",
            object_types=("cert-ldap",),
        ),
        header_types=(("auth", "cert-ldap"),),
        properties=(
            BigipPropertySpec(name="bind-dn", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="bind-pw", value_type="unknown"),
            BigipPropertySpec(name="bind-timeout", value_type="integer"),
            BigipPropertySpec(
                name="check-host-attr",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="check-roles-group",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="filter",
                value_type="reference",
                allow_none=True,
                references=(
                    "apm_url_filter",
                    "net_packet_filter",
                    "net_packet_filter_trusted",
                    "security_packet_filter_default_rules",
                    "security_packet_filter_policy",
                    "security_packet_filter_rule_stat",
                    "sys_air_filter_reset",
                    "sys_log_config_filter",
                ),
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="ignore-auth-info-unavail",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="ignore-unknown-user",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="login-attribute", value_type="reference", allow_none=True),
            BigipPropertySpec(name="login-filter", value_type="string", allow_none=True),
            BigipPropertySpec(name="login-name", value_type="unknown", allow_none=True),
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
            ),
            BigipPropertySpec(name="referrals", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="scope", value_type="enum", enum_values=("base", "one", "sub")),
            BigipPropertySpec(name="search-base-dn", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="search-timeout", value_type="integer"),
            BigipPropertySpec(
                name="servers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="ssl", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="ssl-ca-cert-file", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="ssl-check-peer",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="ssl-ciphers", value_type="string", allow_none=True),
            BigipPropertySpec(name="ssl-client-cert", value_type="string", allow_none=True),
            BigipPropertySpec(name="ssl-client-key", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="ssl-cname-field",
                value_type="enum",
                enum_values=("san-ipadd", "san-other", "san-rid", "san-x400"),
            ),
            BigipPropertySpec(name="ssl-cname-otheroid", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="sso", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(name="version", value_type="integer"),
            BigipPropertySpec(
                name="warnings",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
