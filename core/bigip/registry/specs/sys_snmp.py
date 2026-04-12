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
            "sys_snmp",
            module="sys",
            object_types=("snmp",),
        ),
        header_types=(("sys", "snmp"),),
        properties=(
            BigipPropertySpec(
                name="agent-addresses",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="agent-trap", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="allowed-addresses",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="auth-trap", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="bigip-traps", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="communities",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="access",
                value_type="enum",
                in_sections=("communities",),
                enum_values=("ro", "rw"),
            ),
            BigipPropertySpec(
                name="community-name", value_type="string", in_sections=("communities",)
            ),
            BigipPropertySpec(
                name="description", value_type="string", in_sections=("communities",)
            ),
            BigipPropertySpec(
                name="ipv6",
                value_type="enum",
                in_sections=("communities",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="oid-subset", value_type="string", in_sections=("communities",)),
            BigipPropertySpec(name="source", value_type="string", in_sections=("communities",)),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="disk-monitors",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="description", value_type="string", in_sections=("disk-monitors",)
            ),
            BigipPropertySpec(
                name="minspace", value_type="integer", in_sections=("disk-monitors",)
            ),
            BigipPropertySpec(
                name="minspace-type",
                value_type="enum",
                in_sections=("disk-monitors",),
                enum_values=("percent", "size"),
            ),
            BigipPropertySpec(name="path", value_type="string", in_sections=("disk-monitors",)),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="l2forward-vlan",
                value_type="enum",
                enum_values=("all", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="load-max1", value_type="integer"),
            BigipPropertySpec(name="load-max5", value_type="integer"),
            BigipPropertySpec(name="load-max15", value_type="integer"),
            BigipPropertySpec(
                name="process-monitors",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="description", value_type="string", in_sections=("process-monitors",)
            ),
            BigipPropertySpec(
                name="process", value_type="string", in_sections=("process-monitors",)
            ),
            BigipPropertySpec(
                name="min-processes", value_type="integer", in_sections=("process-monitors",)
            ),
            BigipPropertySpec(
                name="max-processes", value_type="integer", in_sections=("process-monitors",)
            ),
            BigipPropertySpec(
                name="snmpv1", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="snmpv2", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="sys-contact", value_type="string"),
            BigipPropertySpec(name="sys-location", value_type="string"),
            BigipPropertySpec(name="sys-services", value_type="integer"),
            BigipPropertySpec(name="trap-community", value_type="string"),
            BigipPropertySpec(name="trap-source", value_type="string"),
            BigipPropertySpec(
                name="traps",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="auth-password", value_type="string", in_sections=("traps",)),
            BigipPropertySpec(
                name="auth-protocol",
                value_type="enum",
                in_sections=("traps",),
                allow_none=True,
                enum_values=("md5", "sha", "none"),
            ),
            BigipPropertySpec(name="community", value_type="string", in_sections=("traps",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("traps",)),
            BigipPropertySpec(
                name="engine-id", value_type="integer", in_sections=("traps",), allow_none=True
            ),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("traps",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("traps",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="privacy-password", value_type="string", in_sections=("traps",)),
            BigipPropertySpec(
                name="privacy-protocol",
                value_type="enum",
                in_sections=("traps",),
                allow_none=True,
                enum_values=("aes", "des", "none"),
            ),
            BigipPropertySpec(
                name="security-level",
                value_type="enum",
                in_sections=("traps",),
                enum_values=("auth-no-privacy", "auth-privacy", "no-auth-no-privacy"),
            ),
            BigipPropertySpec(name="security-name", value_type="string", in_sections=("traps",)),
            BigipPropertySpec(
                name="version",
                value_type="enum",
                in_sections=("traps",),
                enum_values=("1", "2c", "3"),
            ),
            BigipPropertySpec(
                name="users",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="access", value_type="enum", in_sections=("users",), enum_values=("ro", "rw")
            ),
            BigipPropertySpec(name="auth-password", value_type="string", in_sections=("users",)),
            BigipPropertySpec(
                name="auth-protocol",
                value_type="enum",
                in_sections=("users",),
                allow_none=True,
                enum_values=("md5", "sha", "none"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("users",)),
            BigipPropertySpec(name="oid-subset", value_type="string", in_sections=("users",)),
            BigipPropertySpec(name="privacy-password", value_type="string", in_sections=("users",)),
            BigipPropertySpec(
                name="privacy-protocol",
                value_type="enum",
                in_sections=("users",),
                allow_none=True,
                enum_values=("aes", "des", "none"),
            ),
            BigipPropertySpec(
                name="security-level",
                value_type="enum",
                in_sections=("users",),
                enum_values=("auth-no-privacy", "auth-privacy", "no-auth-no-privacy"),
            ),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                in_sections=("users",),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="v1-traps",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="community", value_type="string", in_sections=("v1-traps",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("v1-traps",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("v1-traps",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("v1-traps",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="v2-traps",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="community", value_type="string", in_sections=("v2-traps",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("v2-traps",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("v2-traps",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("v2-traps",),
                min_value=0,
                max_value=65535,
            ),
        ),
    )
